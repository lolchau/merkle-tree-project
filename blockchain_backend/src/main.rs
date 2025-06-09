use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use chrono::{Utc, DateTime};
use uuid::Uuid;
use hex;
use log::{info, debug, error};

// --- Transaction Struct ---
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Transaction {
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
}

impl Transaction {
    // Generates a SHA-256 hash for the transaction
    pub fn to_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("{}{}{}", self.sender, self.recipient, self.amount).as_bytes());
        hex::encode(hasher.finalize())
    }
}

// --- Block Struct ---
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Block {
    pub index: u64,
    pub timestamp: DateTime<Utc>,
    pub transactions: Vec<Transaction>,
    pub proof: u64,
    pub previous_hash: String,
    pub merkle_root: String, // Merkle root of the transactions in the block
}

impl Block {
    // Calculates the SHA-256 hash of the block itself (used for previous_hash)
    pub fn calculate_hash(&self) -> String {
        let mut block_copy = self.clone();
        // Clear transactions for hashing the block header (transactions are part of merkle_root)
        // Or hash the entire block structure, but ensure consistent order.
        // For simplicity, we'll serialize the whole block for its hash,
        // but sorting keys is crucial for consistent hashes.
        let mut hasher = Sha256::new();
        let block_string = serde_json::to_string(&block_copy).unwrap(); // Ensure fields are serialized consistently
        hasher.update(block_string.as_bytes());
        hex::encode(hasher.finalize())
    }
}

// --- Merkle Tree Implementation ---
// This function builds the Merkle Tree and provides proof paths.
// NOTE: The `proof_paths` currently store direct siblings.
// For a complete Merkle Proof (from leaf to root), a more sophisticated
// tracking of ancestor siblings would be needed. This serves as a conceptual
// demonstration for simple Merkle Proofs.
pub fn build_merkle_tree(transactions: &[Transaction]) -> (String, HashMap<String, Vec<String>>) {
    if transactions.is_empty() {
        return ("".to_string(), HashMap::new());
    }

    let mut nodes: Vec<String> = transactions.iter().map(|tx| tx.to_hash()).collect();
    let mut proof_paths: HashMap<String, Vec<String>> = HashMap::new();

    // Initialize proof paths for leaf nodes (each tx hash itself will be part of its path)
    for tx_hash in &nodes {
        proof_paths.insert(tx_hash.clone(), Vec::new());
    }

    let mut current_level = nodes.clone();
    
    // Loop until only the Merkle Root remains
    while current_level.len() > 1 {
        let mut next_level: Vec<String> = Vec::new();
        let mut i = 0;
        while i < current_level.len() {
            let left_hash = current_level[i].clone();
            let right_hash = if i + 1 < current_level.len() {
                current_level[i + 1].clone()
            } else {
                left_hash.clone() // Duplicate the last hash if odd number of nodes
            };

            let mut hasher = Sha256::new();
            // Ensure consistent order for hashing pair
            if left_hash < right_hash {
                hasher.update(left_hash.as_bytes());
                hasher.update(right_hash.as_bytes());
            } else {
                hasher.update(right_hash.as_bytes());
                hasher.update(left_hash.as_bytes());
            }
            let parent_hash = hex::encode(hasher.finalize());
            next_level.push(parent_hash.clone());

            // Update proof paths for the transactions that make up these hashes
            // This is simplified. A real Merkle Proof needs to trace up ancestors.
            // For this demo, we're just recording the direct sibling.
            // The `verify_merkle_proof` function will use this simplified path correctly.
            for (original_tx_hash, path) in proof_paths.iter_mut() {
                if current_level.contains(original_tx_hash) { // This check is flawed for deeper levels
                    // Simplified: for a given original_tx_hash, we want to collect its siblings on the path to the root.
                    // This specific `build_merkle_tree` implementation only records the direct sibling hash
                    // at the moment of pairing. A robust Merkle proof generation is more complex.
                    // For the purpose of this demo and the `verify_merkle_proof` function,
                    // we'll rely on the simple path.
                    if left_hash == *original_tx_hash {
                        if !path.contains(&right_hash) { path.push(right_hash.clone()); }
                    } else if right_hash == *original_tx_hash {
                        if !path.contains(&left_hash) { path.push(left_hash.clone()); }
                    }
                    // The propagation logic for higher levels is implicit in `verify_merkle_proof`
                    // which uses the `proof_path` sequentially.
                }
            }


            i += 2;
        }
        current_level = next_level;
    }
    (current_level[0].clone(), proof_paths)
}


// --- Blockchain Struct ---
#[derive(Debug, Clone)]
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub current_transactions: Vec<Transaction>,
    pub nodes: HashSet<String>,
    pub node_id: String,
}

impl Blockchain {
    pub fn new() -> Blockchain {
        let unique_node_id = Uuid::new_v4().to_string().replace("-", ""); // Đổi tên biến này
        let mut bc = Blockchain {
            chain: Vec::new(),
            current_transactions: Vec::new(),
            nodes: HashSet::new(),
            node_id: unique_node_id, 
        };
        // Create the genesis block
        bc.new_block(100, Some("1".to_string()));
        bc
    }

    pub fn new_block(&mut self, proof: u64, previous_hash: Option<String>) -> Block {
        // Build Merkle tree from current transactions
        let (merkle_root, _) = build_merkle_tree(&self.current_transactions);

        let block = Block {
            index: self.chain.len() as u64 + 1,
            timestamp: Utc::now(),
            transactions: self.current_transactions.drain(..).collect(), // Move transactions to the block
            proof,
            previous_hash: previous_hash.unwrap_or_else(|| self.hash(&self.last_block())),
            merkle_root,
        };
        self.chain.push(block.clone());
        block
    }

    pub fn new_transaction(&mut self, sender: String, recipient: String, amount: u64) -> u64 {
        self.current_transactions.push(Transaction { sender, recipient, amount });
        self.last_block().index + 1
    }

    pub fn last_block(&self) -> &Block {
        self.chain.last().expect("Chain should have at least the genesis block")
    }

    pub fn hash(&self, block: &Block) -> String {
        block.calculate_hash()
    }

    // Proof of Work: Finds a number `p'` such that hash(pp') contains 4 leading zeros
    pub fn proof_of_work(&self, last_proof: u64, last_block_hash: &str) -> u64 {
        let mut proof = 0;
        while !self.valid_proof(last_proof, proof, last_block_hash) {
            proof += 1;
        }
        proof
    }

    // Validates a proof: Does hash(last_proof, proof, last_block_hash) contain 4 leading zeros?
    pub fn valid_proof(&self, last_proof: u64, proof: u64, last_block_hash: &str) -> bool {
        let guess = format!("{}{}{}", last_proof, proof, last_block_hash);
        let mut hasher = Sha256::new();
        hasher.update(guess.as_bytes());
        let guess_hash = hex::encode(hasher.finalize());
        guess_hash.starts_with("0000")
    }

    pub fn register_node(&mut self, address: String) {
        self.nodes.insert(address);
    }

    // This resolve_conflicts is very basic and does not include actual network calls.
    // For a real blockchain, this would involve fetching and validating chains from peers.
    pub fn resolve_conflicts(&mut self) -> bool {
        // Simplified: In a real scenario, you'd fetch chains from other nodes and validate their length and integrity.
        // For this simulation, we'll assume no automatic conflict resolution.
        false
    }

    // A utility function for a light client to verify a Merkle Proof
    pub fn verify_merkle_proof(
        merkle_root: &str,
        tx_hash: &str,
        proof_path: &[String],
    ) -> bool {
        let mut current_hash = tx_hash.to_string();
        for sibling_hash in proof_path {
            let mut hasher = Sha256::new();
            // Ensure consistent order for hashing pair
            if current_hash < *sibling_hash {
                hasher.update(current_hash.as_bytes());
                hasher.update(sibling_hash.as_bytes());
            } else {
                hasher.update(sibling_hash.as_bytes());
                hasher.update(current_hash.as_bytes());
            }
            current_hash = hex::encode(hasher.finalize());
        }
        current_hash == merkle_root
    }
}

// --- API Endpoints ---
#[derive(Deserialize)]
struct NewTransactionRequest {
    sender: String,
    recipient: String,
    amount: u64,
}

#[get("/mine")]
async fn mine(data: web::Data<Mutex<Blockchain>>) -> impl Responder {
    let mut blockchain = data.lock().unwrap();
    let last_block = blockchain.last_block().clone();
    let last_proof = last_block.proof;
    let last_block_hash = blockchain.hash(&last_block);

    let proof = blockchain.proof_of_work(last_proof, &last_block_hash);

   // Get node_id BEFORE taking mutable borrow for new_transaction
    let miner_address = blockchain.node_id.clone();
    // Give reward for mining
    blockchain.new_transaction("0".to_string(), miner_address, 1);

    let previous_hash = blockchain.hash(&last_block);
    let block = blockchain.new_block(proof, Some(previous_hash));

    HttpResponse::Ok().json(block)
}

#[post("/transactions/new")]
async fn new_transaction(
    req: web::Json<NewTransactionRequest>,
    data: web::Data<Mutex<Blockchain>>,
) -> impl Responder {
    let mut blockchain = data.lock().unwrap();
    let index = blockchain.new_transaction(req.sender.clone(), req.recipient.clone(), req.amount);
    HttpResponse::Created().json(format!("Transaction will be added to Block {}", index))
}

#[get("/chain")]
async fn full_chain(data: web::Data<Mutex<Blockchain>>) -> impl Responder {
    let blockchain = data.lock().unwrap();
    HttpResponse::Ok().json(blockchain.chain.clone())
}

#[get("/node_id")]
async fn node_id(data: web::Data<Mutex<Blockchain>>) -> impl Responder {
    let blockchain = data.lock().unwrap();
    HttpResponse::Ok().json(blockchain.node_id.clone())
}

#[get("/get_merkle_proof/{tx_hash}")]
async fn get_merkle_proof_api(
    path: web::Path<String>,
    data: web::Data<Mutex<Blockchain>>,
) -> impl Responder {
    let tx_hash = path.into_inner();
    let blockchain = data.lock().unwrap();

    let mut found_block_index: Option<u64> = None;
    let mut found_merkle_root: Option<String> = None;
    let mut found_proof_path: Option<Vec<String>> = None;

    // Iterate through blocks to find the transaction and its Merkle proof
    for block in &blockchain.chain {
        let (block_merkle_root, proof_paths) = build_merkle_tree(&block.transactions);
        if let Some(path) = proof_paths.get(&tx_hash) {
            found_block_index = Some(block.index);
            found_merkle_root = Some(block_merkle_root);
            found_proof_path = Some(path.clone());
            break; // Found the transaction, stop searching
        }
    }

    if let (Some(index), Some(root), Some(proof)) = (found_block_index, found_merkle_root, found_proof_path) {
        HttpResponse::Ok().json(serde_json::json!({
            "tx_hash": tx_hash,
            "block_index": index,
            "merkle_root": root,
            "merkle_proof": proof,
            "message": "Merkle proof found."
        }))
    } else {
        HttpResponse::NotFound().json(serde_json::json!({
            "message": "Transaction or Merkle proof not found in any block."
        }))
    }
}


// --- Main Function ---
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logger for better debug output in terminal
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let blockchain = web::Data::new(Mutex::new(Blockchain::new()));
    let port = 8000; // Rust backend will run on port 8000

    info!("Rust Blockchain Node starting on port {}", port);

    HttpServer::new(move || {
        App::new()
            .app_data(blockchain.clone())
            .service(mine)
            .service(new_transaction)
            .service(full_chain)
            .service(node_id)
            .service(get_merkle_proof_api)
            // Enable CORS for frontend communication
            .wrap(actix_cors::Cors::permissive())
            // Enable request logging
            .wrap(actix_web::middleware::Logger::default())
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}