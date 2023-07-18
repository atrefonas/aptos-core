use crate::{
    dag::{
        dag_network::{DAGNetworkSender, RpcWithFallback},
        types::{DAGMessage, TestAck, TestMessage},
    },
    network::TConsensusMsg,
    network_interface::ConsensusMsg,
};
use anyhow::{anyhow, bail};
use aptos_consensus_types::common::Author;
use aptos_infallible::Mutex;
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::validator_verifier::random_validator_verifier;
use async_trait::async_trait;
use claims::{assert_ok, assert_err};
use futures::StreamExt;
use std::{collections::HashMap, sync::Arc, time::Duration};

#[derive(Clone)]
enum TestPeerState {
    Fast,
    Slow(Duration),
    FailSlow(Duration),
}

#[derive(Clone)]
struct MockDAGNetworkSender {
    time_service: TimeService,
    test_peer_state: Arc<Mutex<HashMap<Author, TestPeerState>>>,
}

#[async_trait]
impl DAGNetworkSender for MockDAGNetworkSender {
    async fn send_rpc(
        &self,
        receiver: Author,
        message: ConsensusMsg,
        _timeout: Duration,
    ) -> anyhow::Result<ConsensusMsg> {
        let message: TestMessage = (TConsensusMsg::from_network_message(message)
            as anyhow::Result<DAGMessage>)?
            .try_into()?;
        let state = {
            self.test_peer_state
                .lock()
                .get(&receiver)
                .ok_or_else(|| anyhow!("lookup failed"))?
                .clone()
        };
        match state {
            TestPeerState::Fast => Ok(DAGMessage::from(TestAck(message.0)).into_network_message()),
            TestPeerState::Slow(duration) => {
                self.time_service.sleep(duration.clone()).await;
                Ok(DAGMessage::from(TestAck(message.0)).into_network_message())
            },
            TestPeerState::FailSlow(duration) => {
                self.time_service.sleep(duration.clone()).await;
                bail!("failed to respond");
            },
        }
    }

    async fn send_rpc_with_fallbacks(
        &self,
        responders: Vec<Author>,
        message: ConsensusMsg,
        timeout: Duration,
    ) -> RpcWithFallback {
        RpcWithFallback::new(
            responders,
            message,
            timeout,
            Arc::new(self.clone()),
            self.time_service.clone(),
        )
    }
}

#[tokio::test]
async fn test_send_rpc_with_fallback() {
    let (_, validator_verifier) = random_validator_verifier(5, None, false);
    let validators = validator_verifier.get_ordered_account_addresses();
    let time_service = TimeService::real();

    let sender = MockDAGNetworkSender {
        time_service: time_service.clone(),
        test_peer_state: Arc::new(Mutex::new(HashMap::from([
            (validators[0], TestPeerState::Fast),
            (
                validators[1],
                TestPeerState::FailSlow(Duration::from_secs(1)),
            ),
            (validators[2], TestPeerState::Slow(Duration::from_secs(5))),
            (
                validators[3],
                TestPeerState::FailSlow(Duration::from_secs(3)),
            ),
            (validators[4], TestPeerState::Slow(Duration::from_secs(2))),
        ]))),
    };
    
    let message = TestMessage(vec![42; validators.len() - 1]);
    let mut rpc = sender
        .send_rpc_with_fallbacks(
            validators,
            DAGMessage::from(message).into_network_message(),
            Duration::from_millis(100),
        )
        .await;

    assert_ok!(rpc.next().await.unwrap());
    assert_err!(rpc.next().await.unwrap());
    assert_ok!(rpc.next().await.unwrap());
    assert_err!(rpc.next().await.unwrap());
    assert_ok!(rpc.next().await.unwrap());
}
