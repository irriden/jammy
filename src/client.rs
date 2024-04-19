pub struct Client(fedimint_tonic_lnd::Client);

use fedimint_tonic_lnd::lnrpc::Payment;
use fedimint_tonic_lnd::tonic::codec::Streaming;
use std::sync::Arc;

#[allow(dead_code)]
impl Client {
    pub async fn get_pubkey(&mut self) -> String {
        self.0
            .lightning()
            .get_info(fedimint_tonic_lnd::lnrpc::GetInfoRequest {})
            .await
            .unwrap()
            .into_inner()
            .identity_pubkey
    }

    pub async fn graph_get_node_peers(&mut self, node_pubkey: String) -> Vec<String> {
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

    pub async fn open_channel(
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

    pub async fn add_invoice(&mut self, value: i64) -> String {
        self.0
            .lightning()
            .add_invoice(fedimint_tonic_lnd::lnrpc::Invoice {
                value,
                ..Default::default()
            })
            .await
            .unwrap()
            .into_inner()
            .payment_request
    }

    pub async fn add_hold_invoice(&mut self, hash: Vec<u8>, value: i64) -> String {
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

    pub async fn send_payment(&mut self, payment_request: String) -> Streaming<Payment> {
        self.0
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
            .into_inner()
    }

    pub async fn on_payment_result(
        &mut self,
        mut stream: Streaming<Payment>,
        on_success: Arc<dyn Fn(Payment) + Send + Sync>,
    ) {
        tokio::task::spawn(async move {
            while let Some(payment) = stream.message().await.unwrap() {
                on_success(payment);
            }
        });
    }

    pub async fn print_payment_result(&mut self, stream: Streaming<Payment>) {
        self.on_payment_result(
            stream,
            Arc::new(|payment| {
                if payment.status == 3 {
                    println!("payment failed!");
                } else if payment.status == 2 {
                    println!("payment success!");
                }
            }),
        )
        .await;
    }

    pub async fn settle_invoice(&mut self, preimage: Vec<u8>) {
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

    pub async fn lookup_invoice(&mut self, r_hash: Vec<u8>) {
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

    pub async fn connect_peer(&mut self, pubkey: String, host: String) {
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

    pub async fn new_address(&mut self) -> String {
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

    pub async fn cancel_invoice(&mut self, payment_hash: Vec<u8>) {
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

pub fn gen_hash_table(n: usize) -> Vec<([u8; 32], [u8; 32])> {
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

pub async fn new_client(
    rpcserver: &str,
    cert: &str,
    macaroon: &str,
) -> Result<Client, Box<dyn std::error::Error>> {
    let client = Client(
        fedimint_tonic_lnd::connect(format!("https://{}:10009", rpcserver), cert, macaroon).await?,
    );
    Ok(client)
}
