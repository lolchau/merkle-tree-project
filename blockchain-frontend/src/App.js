// File: blockchain-frontend/src/App.js
import React, { useState, useEffect } from 'react';
import './App.css';

// Import sha256 for client-side hashing and verification
const sha256 = require('js-sha256').sha256;

// Define the API URL for your Rust backend
const API_URL = 'http://127.0.0.1:8000'; // Ensure this matches your Rust backend port

function App() {
  const [chain, setChain] = useState([]);
  const [sender, setSender] = useState('Alice');
  const [recipient, setRecipient] = useState('Bob');
  const [amount, setAmount] = useState(10);
  const [message, setMessage] = useState('');
  const [txHashForProof, setTxHashForProof] = useState('');
  const [merkleProofResult, setMerkleProofResult] = useState(null);
  const [nodeId, setNodeId] = useState('');

  // Fetch node ID and blockchain chain on component mount
  useEffect(() => {
    fetchNodeId();
    fetchChain();
  }, []);

  // Fetches the unique ID of the blockchain node (backend)
  const fetchNodeId = async () => {
    try {
      const response = await fetch(`${API_URL}/node_id`);
      const data = await response.json();
      setNodeId(data);
    } catch (error) {
      console.error('Error fetching node ID:', error);
      showMessage('Error fetching node ID. Is the backend running?', true);
    }
  };

  // Fetches the entire blockchain chain from the backend
  const fetchChain = async () => {
    try {
      const response = await fetch(`${API_URL}/chain`);
      const data = await response.json();
      setChain(data);
      showMessage(`Chain loaded successfully. Total ${data.length} blocks.`);
    } catch (error) {
      console.error('Error fetching chain:', error);
      showMessage('Error fetching chain. Is the backend running?', true);
    }
  };

  // Creates a new transaction and sends it to the backend
  const createTransaction = async () => {
    if (!sender || !recipient || isNaN(amount) || amount <= 0) {
      showMessage('Please fill in all transaction details correctly.', true);
      return;
    }

    try {
      const response = await fetch(`${API_URL}/transactions/new`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ sender, recipient, amount: parseInt(amount) })
      });
      const data = await response.json();
      showMessage(data);
      // Note: Transaction is added to pending list, not yet in a block.
      // It will appear in a block after mining.
    } catch (error) {
      console.error('Error creating transaction:', error);
      showMessage('Error creating transaction. Check backend.', true);
    }
  };

  // Mines a new block by sending a request to the backend
  const mineBlock = async () => {
    try {
      const response = await fetch(`${API_URL}/mine`);
      const data = await response.json();
      showMessage(`New Block Forged! Index: ${data.index}`);
      fetchChain(); // Refresh chain display after mining
    } catch (error) {
      console.error('Error mining block:', error);
      showMessage('Error mining block. Check backend.', true);
    }
  };

  // Fetches the Merkle Proof for a given transaction hash from the backend
  const getMerkleProof = async () => {
    if (!txHashForProof) {
      showMessage('Please enter a transaction hash to get its Merkle proof.', true);
      setMerkleProofResult(null);
      return;
    }
    setMerkleProofResult(null); // Clear previous result

    try {
      const response = await fetch(`<span class="math-inline">\{API\_URL\}/get\_merkle\_proof/</span>{txHashForProof}`);
      if (response.status === 404) {
        showMessage('Transaction or Merkle proof not found in any block. Did you mine it?', true);
        setMerkleProofResult(null);
        return;
      }
      const data = await response.json();
      setMerkleProofResult(data);
      showMessage('Merkle proof fetched. Now you can verify it client-side.');
    } catch (error) {
      console.error('Error fetching Merkle proof:', error);
      showMessage('Error fetching Merkle proof. Check backend or hash format.', true);
      setMerkleProofResult(null);
    }
  };

  // Displays messages to the user (success or error)
  const showMessage = (msg, isError = false) => {
    setMessage(msg);
    const messageDiv = document.getElementById('message');
    if (messageDiv) {
      messageDiv.style.backgroundColor = isError ? '#f8d7da' : '#d4edda';
      messageDiv.style.color = isError ? '#721c24' : '#155724';
      messageDiv.style.display = 'block';
    }
    setTimeout(() => {
      if (messageDiv) messageDiv.style.display = 'none';
    }, 5000); // Hide message after 5 seconds
  };

  // Helper function to calculate transaction hash on frontend
  const calculateTxHash = (tx) => {
    return sha256(`<span class="math-inline">\{tx\.sender\}</span>{tx.recipient}${tx.amount}`);
  };

  // Client-side verification of Merkle Proof
  const verifyMerkleProofClientSide = () => {
    if (!merkleProofResult) {
        showMessage('No Merkle proof data to verify. Please fetch one first.', true);
        return;
    }

    const { tx_hash, merkle_root, merkle_proof } = merkleProofResult;
    let currentHash = tx_hash;

    // Recompute the root using the provided proof path
    // This logic must exactly mirror the Rust backend's Merkle Proof verification
    for (const siblingHash of merkle_proof) {
        let hasher = sha256.create();
        // Ensure consistent hashing order as in backend (lexicographical comparison)
        if (currentHash < siblingHash) {
            hasher.update(currentHash);
            hasher.update(siblingHash);
        } else {
            hasher.update(siblingHash);
            hasher.update(currentHash);
        }
        currentHash = hasher.hex();
    }

    if (currentHash === merkle_root) {
        showMessage('Merkle Proof verified successfully! The transaction exists in the block.', false);
    } else {
        showMessage('Merkle Proof verification FAILED! This transaction might not be in the block, or the proof is invalid.', true);
    }
  };


  return (
    <div className="App">
      <div className="container">
        <h1>Rust Blockchain DApp</h1>
        <p>Node ID: <strong>{nodeId}</strong></p>
        <p>Connected to Rust Backend API on: {API_URL}</p>

        <hr />

        <div className="section">
          <h2>View Blockchain</h2>
          <button onClick={fetchChain}>Refresh Chain</button>
          <div id="message" style={{ display: 'none', marginTop: '15px', padding: '10px', borderRadius: '4px' }}>{message}</div>
          <div className="chain-display">
            {chain.length > 0 ? (
              chain.map(block => (
                <div key={block.index} className="block">
                  <h3>Block #{block.index}</h3>
                  <p><strong>Timestamp:</strong> {new Date(block.timestamp).toLocaleString()}</p>
                  <p><strong>Proof:</strong> {block.proof}</p>
                  <p><strong>Previous Hash:</strong> {block.previous_hash}</p>
                  <p><strong>Merkle Root:</strong> {block.merkle_root}</p>
                  <h4>Transactions:</h4>
                  <ul className="transactions-list">
                    {block.transactions && block.transactions.length > 0 ? (
                      block.transactions.map((tx, idx) => (
                        <li key={idx}>
                          <strong>Sender:</strong> {tx.sender} <br />
                          <strong>Recipient:</strong> {tx.recipient} <br />
                          <strong>Amount:</strong> {tx.amount} <br />
                          <strong>Hash:</strong> <code title="Copy this hash for Merkle Proof">{calculateTxHash(tx)}</code>
                        </li>
                      ))
                    ) : (
                      <li>No transactions in this block.</li>
                    )}
                  </ul>
                </div>
              ))
            ) : (
              <p>No blocks in the chain yet. Mine one!</p>
            )}
          </div>
        </div>

        <hr />

        <div className="section">
          <h2>Create New Transaction</h2>
          <label>Sender:</label>
          <input type="text" value={sender} onChange={(e) => setSender(e.target.value)} />

          <label>Recipient:</label>
          <input type="text" value={recipient} onChange={(e) => setRecipient(e.target.value)} />

          <label>Amount:</label>
          <input type="number" value={amount} onChange={(e) => setAmount(e.target.value)} />

          <button onClick={createTransaction}>Create Transaction</button>
        </div>

        <hr />

        <div className="section">
          <h2>Mine New Block</h2>
          <button onClick={mineBlock}>Mine Block</button>
        </div>

        <hr />

        <div className="section">
            <h2>Merkle Proof for Light Client</h2>
            <p>Enter a Transaction Hash to get its Merkle Proof:</p>
            <label>Transaction Hash:</label>
            <input
                type="text"
                value={txHashForProof}
                onChange={(e) => setTxHashForProof(e.target.value)}
                placeholder="e.g., d4e5f6g7..."
            />
            <button onClick={getMerkleProof}>Get Merkle Proof</button>

            {merkleProofResult && (
                <div className="merkle-proof-result">
                    <h4>Merkle Proof Details:</h4>
                    <p><strong>Transaction Hash:</strong> {merkleProofResult.tx_hash}</p>
                    <p><strong>Block Index:</strong> {merkleProofResult.block_index}</p>
                    <p><strong>Block Merkle Root:</strong> {merkleProofResult.merkle_root}</p>
                    <p><strong>Merkle Proof Path:</strong></p>
                    <pre>{JSON.stringify(merkleProofResult.merkle_proof, null, 2)}</pre>
                    <button onClick={verifyMerkleProofClientSide}>Verify Merkle Proof (Client-side)</button>
                </div>
            )}
        </div>
      </div>
    </div>
  );
}

export default App;