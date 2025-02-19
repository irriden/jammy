use tokio::time::{sleep, Duration};

const LND_0_RPCSERVER: &str = env!("LND_0_RPCSERVER");
const LND_0_CERT: &str = env!("LND_0_CERT");
const LND_0_MACAROON: &str = env!("LND_0_MACAROON");

const LND_1_RPCSERVER: &str = env!("LND_1_RPCSERVER");
const LND_1_CERT: &str = env!("LND_1_CERT");
const LND_1_MACAROON: &str = env!("LND_1_MACAROON");

const _LND_2_RPCSERVER: &str = env!("LND_2_RPCSERVER");
const _LND_2_CERT: &str = env!("LND_2_CERT");
const _LND_2_MACAROON: &str = env!("LND_2_MACAROON");

const TARGET: &str = env!("TARGET");

#[tokio::main]
async fn main() {
    let mut alice = Client(
        fedimint_tonic_lnd::connect(
            format!("https://{}:10009", LND_0_RPCSERVER),
            LND_0_CERT,
            LND_0_MACAROON,
        )
        .await
        .unwrap(),
    );
    let mut bob = Client(
        fedimint_tonic_lnd::connect(
            format!("https://{}:10009", LND_1_RPCSERVER),
            LND_1_CERT,
            LND_1_MACAROON,
        )
        .await
        .unwrap(),
    );


    let target_peers = alice.graph_get_node_peers(String::from(TARGET)).await;
    alice
        .open_channel(target_peers[1].clone(), 500_000, 0)
        .await;
    bob.open_channel(target_peers[2].clone(), 500_000, 250_000)
        .await;
    println!("Please confirm the channels!");
    std::io::stdin().read_line(&mut String::new()).unwrap();

    let hash_table = gen_hash_table(10);

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

struct Client(fedimint_tonic_lnd::Client);

#[allow(dead_code)]
impl Client {
    async fn get_pubkey(&mut self) -> String {
        self.0
            .lightning()
            .get_info(fedimint_tonic_lnd::lnrpc::GetInfoRequest {})
            .await
            .unwrap()
            .into_inner()
            .identity_pubkey
    }

    async fn graph_get_node_peers(&mut self, node_pubkey: String) -> Vec<String> {
        let channels = self
            .0
            .lightning()
            .get_node_info(fedimint_tonic_lnd::lnrpc::NodeInfoRequest {
                pub_key: node_pubkey.clone(),
                include_channels: true,
            })
            .await
            .unwrap()
            .into_inner()
            .channels;
        channels
            .iter()
            .map(|channel| {
                if channel.node1_pub == node_pubkey {
                    channel.node2_pub.clone()
                } else {
                    channel.node1_pub.clone()
                }
            })
            .collect()
    }

    async fn open_channel(
        &mut self,
        node_pubkey: String,
        local_funding_amount: i64,
        push_sat: i64,
    ) {
        use fedimint_tonic_lnd::lnrpc::channel_point::FundingTxid;
        let res = self
            .0
            .lightning()
            .open_channel_sync(fedimint_tonic_lnd::lnrpc::OpenChannelRequest {
                node_pubkey: hex::decode(&node_pubkey).unwrap(),
                local_funding_amount,
                push_sat,
                ..Default::default()
            })
            .await
            .unwrap()
            .into_inner();
        let s = match res.funding_txid.unwrap() {
            FundingTxid::FundingTxidBytes(b) => hex::encode(b),
            FundingTxid::FundingTxidStr(_s) => unreachable!(),
        };
        println!("{}", s);
    }

    async fn add_hold_invoice(&mut self, hash: Vec<u8>, value: i64) -> String {
        self.0
            .invoices()
            .add_hold_invoice(fedimint_tonic_lnd::invoicesrpc::AddHoldInvoiceRequest {
                hash,
                value,
                ..Default::default()
            })
            .await
            .unwrap()
            .into_inner()
            .payment_request
    }

    async fn send_payment(&mut self, payment_request: String) {
        let mut stream = self
            .0
            .router()
            .send_payment_v2(fedimint_tonic_lnd::routerrpc::SendPaymentRequest {
                payment_request,
                fee_limit_sat: 100_000,
                timeout_seconds: 100_000,
                endorsed: 1i32,
                ..Default::default()
            })
            .await
            .unwrap()
            .into_inner();
        tokio::task::spawn(async move {
            while let Some(payment) = stream.message().await.unwrap() {
                if payment.status == 3 {
                    println!("payment failed!");
                } else if payment.status == 2 {
                    println!("payment success!");
                }
            }
        });
    }

    async fn settle_invoice(&mut self, preimage: Vec<u8>) {
        let _res = self
            .0
            .invoices()
            .settle_invoice(fedimint_tonic_lnd::invoicesrpc::SettleInvoiceMsg { preimage })
            .await
            .unwrap()
            .into_inner();
        //println!("{:?}", res);
    }

    async fn subscribe_invoices(&mut self) {
        let mut invoice_stream = self
            .0
            .lightning()
            .subscribe_invoices(fedimint_tonic_lnd::lnrpc::InvoiceSubscription {
                add_index: 0,
                settle_index: 0,
            })
            .await
            .expect("Failed to call subscribe_invoices")
            .into_inner();

        tokio::task::spawn(async move {
            while let Some(invoice) = invoice_stream
                .message()
                .await
                .expect("Failed to receive invoices")
            {
                let htlcs = invoice.htlcs;
                for htlc in htlcs {
                    if htlc.incoming_endorsed {
                        println!("HTLC endorsed!!");
                    } else {
                        println!("not endorsed");
                    }
                }
            }
        });
    }

    async fn lookup_invoice(&mut self, r_hash: Vec<u8>) {
        let htlcs = self
            .0
            .lightning()
            .lookup_invoice(fedimint_tonic_lnd::lnrpc::PaymentHash {
                r_hash,
                ..Default::default()
            })
            .await
            .unwrap()
            .into_inner()
            .htlcs;
        for htlc in htlcs {
            if htlc.incoming_endorsed {
                println!("HTLC endorsed!!");
            } else {
                println!("not endorsed");
            }
        }
    }

    async fn connect_peer(&mut self, pubkey: String, host: String) {
        use fedimint_tonic_lnd::lnrpc::LightningAddress;
        let _ = self
            .0
            .lightning()
            .connect_peer(fedimint_tonic_lnd::lnrpc::ConnectPeerRequest {
                addr: Some(LightningAddress { pubkey, host }),
                ..Default::default()
            })
            .await;
    }

    async fn new_address(&mut self) -> String {
        self.0
            .lightning()
            .new_address(fedimint_tonic_lnd::lnrpc::NewAddressRequest {
                ..Default::default()
            })
            .await
            .unwrap()
            .into_inner()
            .address
    }

    async fn cancel_invoice(&mut self, payment_hash: Vec<u8>) {
        let _res = self
            .0
            .invoices()
            .cancel_invoice(fedimint_tonic_lnd::invoicesrpc::CancelInvoiceMsg { payment_hash })
            .await
            .unwrap()
            .into_inner();
        //println!("{:?}", res);
    }
}

fn gen_hash_table(n: usize) -> Vec<([u8; 32], [u8; 32])> {
    use bitcoin_hashes::sha256;
    use bitcoin_hashes::Hash;
    use rand::{thread_rng, Rng};
    let mut rng = thread_rng();
    let mut hash_table = Vec::with_capacity(n);
    for _ in 0..n {
        let mut preimage = [0u8; 32];
        rng.fill(&mut preimage[..]);
        let hash = sha256::Hash::hash(&preimage);
        hash_table.push((preimage, hash.to_byte_array()));
    }
    hash_table
}
