/// Copyright (c) 2022 Tetherion

use {
    crate::{
        block::Block,
        tetherion::Tetherion
    },
    libp2p::{
        floodsub::{Floodsub, FloodsubEvent, Topic},
        identity,
        mdns::{Mdns, MdnsEvent},
        swarm::{NetworkBehaviourEventProcess, Swarm},
        NetworkBehaviour,
        PeerId
    }
};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio::sync::mpsc;

pub static KEYS: Lazy<identity::Keypair> = Lazy::new(identity::Keypair::generate_ed25519);
pub static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
pub static CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("chains"));
pub static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("blocks"));

#[derive(Serialize, Deserialize, Debug)]
pub struct ChainResponse {
    pub tetherion: Tetherion<String>,
    pub receiver: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LocalChainRequest {
    pub from_peer_id: String
}

pub enum EventType {
    LocalChainResponse(ChainResponse),
    Input(String),
    Init,
}

#[derive(NetworkBehaviour)]
pub struct TetherionBehaviour {
    pub floodsub: Floodsub,
    pub mdns: Mdns,

    #[behaviour(ignore)]
    pub response_sender: mpsc::UnboundedSender<ChainResponse>,

    #[behaviour(ignore)]
    pub init_sender: mpsc::UnboundedSender<bool>,

    #[behaviour(ignore)]
    pub tetherion: Tetherion<String>
}

impl TetherionBehaviour {
    pub async fn new(
        tetherion: Tetherion<String>,
        response_sender: mpsc::UnboundedSender<ChainResponse>,
        init_sender: mpsc::UnboundedSender<bool>,
    ) -> Self {
        let mut behaviour = Self {
            floodsub: Floodsub::new(*PEER_ID),
            mdns: Mdns::new(Default::default())
                .await
                .expect("MDNS should be created"),
            response_sender,
            init_sender,
            tetherion
        };
        behaviour.floodsub.subscribe(CHAIN_TOPIC.clone());
        behaviour.floodsub.subscribe(BLOCK_TOPIC.clone());

        behaviour
    }

    /// Checks whether remote blockchain is worse than the local one:
    /// 1. by the validity
    /// 2. in case both blockchains are valid, by the length
    /// 3. in case both blockchains are of the same length, by the olderness
    fn is_better_than(&self, remote: &Tetherion<String>) -> bool {
        match (self.tetherion.is_valid(), remote.is_valid()) {
            (Ok(()), Ok(())) => {
                if self.tetherion.blocks().len() == remote.blocks().len() {
                    return self.tetherion.creation_timestamp() <= remote.creation_timestamp();
                }
                self.tetherion.blocks().len() >= remote.blocks().len()
            },
            (Ok(()), Err(err)) => {
                log::debug!("Remote blockchain is invalid: {}", err);
                true
            },
            (Err(err), Ok(())) => {
                log::debug!("Local blockchain is invalid: {}", err);
                false
            },
            (Err(local_err), Err(remote_err)) => {
                panic!("Local blockchain is invalid: {}, remote blockchain is invalid: {}", local_err, remote_err);
            }
        }
    }
}

// incoming event handler
impl NetworkBehaviourEventProcess<FloodsubEvent> for TetherionBehaviour {
    fn inject_event(&mut self, event: FloodsubEvent) {
        if let FloodsubEvent::Message(msg) = event {
            if let Ok(resp) = serde_json::from_slice::<ChainResponse>(&msg.data) {
                if resp.receiver == PEER_ID.to_string() {
                    log::info!("Response from {}:", msg.source);

                    if !self.is_better_than(&resp.tetherion) {
                        self.tetherion = resp.tetherion;
                    }
                }
            } else if let Ok(resp) = serde_json::from_slice::<LocalChainRequest>(&msg.data) {
                log::info!("sending local chain to {}", msg.source.to_string());
                if resp.from_peer_id == PEER_ID.to_string() {
                    if let Err(e) = self.response_sender.send(ChainResponse {
                        tetherion: self.tetherion.clone(),
                        receiver: msg.source.to_string(),
                    }) {
                        log::error!("error sending response via channel, {}", e);
                    }
                }
            } else if let Ok(block) = serde_json::from_slice::<Block<String>>(&msg.data) {
                log::info!("received new block from {}", msg.source.to_string());
                match self.tetherion.add_block(block) {
                    Ok(()) => (),
                    Err(err) => log::error!("Error {}", err)
                }
            }
        }
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for TetherionBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(discovered_list) => {
                for (peer, _addr) in discovered_list {
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            MdnsEvent::Expired(expired_list) => {
                for (peer, _addr) in expired_list {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}

pub fn get_peers(swarm: &Swarm<TetherionBehaviour>) -> Vec<String> {
    let nodes = swarm.behaviour().mdns.discovered_nodes();
    let mut unique_peers = HashSet::new();
    for peer in nodes {
        unique_peers.insert(peer);
    }
    unique_peers.iter().map(|p| p.to_string()).collect()
}

pub fn handle_print_peers(swarm: &Swarm<TetherionBehaviour>) {
    log::info!("Peers:");
    let peers = get_peers(swarm);
    peers.iter().for_each(|p| log::info!("{}", p));
}

pub fn handle_print_chain(swarm: &Swarm<TetherionBehaviour>) {
    log::info!("Local Tetherion blockchain:");
    let json = serde_json::to_string_pretty(
        &swarm.behaviour().tetherion.blocks()
    ).expect("Blocks should be jsonified");
    log::info!("{}", json);
}

pub fn handle_create_block(cmd: &str, swarm: &mut Swarm<TetherionBehaviour>) {
    if let Some(data) = cmd.strip_prefix("create b") {
        let behaviour = swarm.behaviour_mut();
        let latest_block = behaviour
            .tetherion
            .blocks()
            .last()
            .expect("there is at least one block");
        let block = Block::<String>::new(
            latest_block.id + 1,
            &latest_block.hash,
            data.to_owned(),
            behaviour.tetherion.difficulty()
        );
        let json = serde_json::to_string(&block).expect("can jsonify request");
        match behaviour.tetherion.add_block(block) {
            Ok(()) => {
                log::info!("broadcasting new block");
                behaviour
                    .floodsub
                    .publish(BLOCK_TOPIC.clone(), json.as_bytes());
            },
            Err(err) => log::error!("{}", err)
        }
    }
}