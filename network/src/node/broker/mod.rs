//
// MIT License
//
// Copyright (c) 2018 Stegos
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
//!
//! Message broker
//!

use failure::Error;
use fnv::FnvHashMap;
use futures::sync::mpsc;
use futures::Stream;
use futures::{Async, Future, Poll};
use libp2p::floodsub::{self, TopicHash};
use log::*;
use protobuf::Message as ProtoMessage;
use stegos_crypto::pbc::secure;

// Autogenerated protobuf bindings
mod unicast;

// ----------------------------------------------------------------
// Public API.
// ----------------------------------------------------------------

/// Manages subscriptions to topics
///
#[derive(Clone, Debug)]
pub struct Broker {
    pub upstream: mpsc::UnboundedSender<PubsubMessage>,
}

impl Broker {
    /// Create a new Broker service
    pub fn new(
        local_pkey: secure::PublicKey,
        input: floodsub::FloodSubReceiver,
        floodsub_ctl: floodsub::FloodSubController,
    ) -> (impl Future<Item = (), Error = ()>, Broker) {
        let (tx, rx) = mpsc::unbounded();

        let service = BrokerService::new(local_pkey, input, floodsub_ctl, rx);
        let broker = Broker { upstream: tx };
        (service, broker)
    }

    /// Subscribe to topic, returns Stream<Vec<u8>> of messages incoming to topic
    pub fn subscribe<S>(&self, topic: &S) -> Result<mpsc::UnboundedReceiver<Vec<u8>>, Error>
    where
        S: Into<String> + Clone,
    {
        let topic: String = topic.clone().into();
        let (tx, rx) = mpsc::unbounded();
        let msg = PubsubMessage::Subscribe { topic, handler: tx };
        self.upstream.unbounded_send(msg)?;
        Ok(rx)
    }
    /// Published message to topic
    pub fn publish<S>(&self, topic: &S, data: Vec<u8>) -> Result<(), Error>
    where
        S: Into<String> + Clone,
    {
        let topic: String = topic.clone().into();
        let msg = PubsubMessage::Publish {
            topic: topic.clone().into(),
            data,
        };
        self.upstream.unbounded_send(msg)?;
        Ok(())
    }
    // Subscribe to unicast messages
    pub fn subscribe_unicast(&self) -> Result<mpsc::UnboundedReceiver<Vec<u8>>, Error> {
        let (tx, rx) = mpsc::unbounded();
        let msg = PubsubMessage::SubscribeUnicast { consumer: tx };
        self.upstream.unbounded_send(msg)?;
        Ok(rx)
    }

    // Send direct message to public key
    pub fn send(&self, to: secure::PublicKey, data: Vec<u8>) -> Result<(), Error> {
        let msg = PubsubMessage::SendUnicast { to, data };
        self.upstream.unbounded_send(msg)?;
        Ok(())
    }
}

// ----------------------------------------------------------------
// Internal Implementation.
// ----------------------------------------------------------------

const UNICAST_TOPIC: &'static str = "stegos-unicast";

#[derive(Clone, Debug)]
pub enum PubsubMessage {
    Subscribe {
        topic: String,
        handler: mpsc::UnboundedSender<Vec<u8>>,
    },
    Publish {
        topic: String,
        data: Vec<u8>,
    },
    SendUnicast {
        to: secure::PublicKey,
        data: Vec<u8>,
    },
    SubscribeUnicast {
        consumer: mpsc::UnboundedSender<Vec<u8>>,
    },
}

enum Message {
    Pubsub(PubsubMessage),
    Input(floodsub::Message),
}

struct BrokerService {
    local_pkey: secure::PublicKey,
    consumers: FnvHashMap<TopicHash, Vec<mpsc::UnboundedSender<Vec<u8>>>>,
    unicast_consumers: Vec<mpsc::UnboundedSender<Vec<u8>>>,
    pubsub_rx: Box<dyn Stream<Item = Message, Error = ()> + Send>,
    floodsub_ctl: floodsub::FloodSubController,
}

impl BrokerService {
    fn new(
        local_pkey: secure::PublicKey,
        input: floodsub::FloodSubReceiver,
        floodsub_ctl: floodsub::FloodSubController,
        rx: mpsc::UnboundedReceiver<PubsubMessage>,
    ) -> BrokerService {
        let messages =
            rx.map(|m| Message::Pubsub(m))
                .select(input.map(|m| Message::Input(m)).map_err(|e| {
                    error!("Error reading from floodsub receiver: {}", e);
                }));

        let new_topic = floodsub::TopicBuilder::new(UNICAST_TOPIC).build();
        floodsub_ctl.subscribe(&new_topic);

        let service = BrokerService {
            local_pkey,
            consumers: FnvHashMap::default(),
            unicast_consumers: Vec::new(),
            // input,
            // downstream: rx,
            pubsub_rx: Box::new(messages),
            floodsub_ctl,
        };

        service
    }
}

impl Future for BrokerService {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match self.pubsub_rx.poll() {
                Ok(Async::Ready(msg)) => match msg {
                    Some(Message::Pubsub(m)) => match m {
                        PubsubMessage::Subscribe { topic, handler } => {
                            debug!("Subscribed to topic '{}'*", &topic);
                            let new_topic = floodsub::TopicBuilder::new(topic).build();
                            let topic_hash = new_topic.hash();
                            self.consumers
                                .entry(topic_hash.clone())
                                .or_insert(vec![])
                                .push(handler);
                            self.floodsub_ctl.subscribe(&new_topic);
                        }
                        PubsubMessage::Publish { topic, data } => {
                            let new_topic = floodsub::TopicBuilder::new(topic).build();
                            let topic_hash = new_topic.hash();
                            debug!(
                                "Got publish message from Upstream, publishing to topic {}!",
                                topic_hash.clone().into_string()
                            );
                            self.floodsub_ctl.publish(&new_topic, data);
                        }
                        PubsubMessage::SubscribeUnicast { consumer } => {
                            self.unicast_consumers.push(consumer);
                        }
                        PubsubMessage::SendUnicast { to, data } => {
                            debug!("Sending unicast message to: {}", to);
                            let new_topic = floodsub::TopicBuilder::new(UNICAST_TOPIC).build();
                            let msg = encode_unicast(self.local_pkey.clone(), to, data);

                            self.floodsub_ctl.publish(&new_topic, msg);
                        }
                    },
                    Some(Message::Input(m)) => {
                        let topic = floodsub::TopicBuilder::new(UNICAST_TOPIC).build();
                        let unicast_topic_hash = topic.hash();

                        if m.topics.iter().any(|t| t == unicast_topic_hash) {
                            match decode_unicast(m.data.clone()) {
                                Ok((from, to, data)) => {
                                    // send unicast message upstream
                                    if to == self.local_pkey {
                                        debug!(
                                            "Received unicast message from: {}\n\tdata: {}",
                                            from,
                                            String::from_utf8_lossy(&data)
                                        );
                                        self.unicast_consumers.retain({
                                            move |c| {
                                                if let Err(e) = c.unbounded_send(data.clone()) {
                                                    error!("Error sending data to consumer: {}", e);
                                                    false
                                                } else {
                                                    true
                                                }
                                            }
                                        })
                                    }
                                }
                                Err(e) => error!("Failure decoding unicast message: {}", e),
                            }
                        }
                        for t in m.topics.into_iter() {
                            debug!(
                                "Got message for topic {}, sending to consumers",
                                t.clone().into_string()
                            );
                            let consumers = self.consumers.entry(t).or_insert(vec![]);
                            consumers.retain({
                                let data = &m.data;
                                move |c| {
                                    if let Err(e) = c.unbounded_send(data.clone()) {
                                        error!("Error sending data to consumer: {}", e);
                                        false
                                    } else {
                                        true
                                    }
                                }
                            })
                        }
                    }
                    None => return Ok(Async::Ready(())), // All streams are done!
                },
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(e) => {
                    error!("Error in Broker Future: {:?}", e);
                    return Err(());
                }
            }
        }
    }
}

// Encode unicast message
fn encode_unicast(from: secure::PublicKey, to: secure::PublicKey, data: Vec<u8>) -> Vec<u8> {
    let mut msg = unicast::Message::new();
    msg.set_from(from.into_bytes().to_vec());
    msg.set_to(to.into_bytes().to_vec());
    msg.set_data(data);

    msg.write_to_bytes()
        .expect("protobuf encoding should never fail")
}

fn decode_unicast(
    input: Vec<u8>,
) -> Result<(secure::PublicKey, secure::PublicKey, Vec<u8>), Error> {
    let mut msg: unicast::Message = protobuf::parse_from_bytes(&input)?;

    let from = secure::PublicKey::try_from_bytes(&msg.take_from().to_vec())?;
    let to = secure::PublicKey::try_from_bytes(&msg.take_to().to_vec())?;
    let data = msg.take_data().to_vec();

    Ok((from, to, data))
}

#[cfg(test)]
mod tests {
    use stegos_crypto::pbc::secure;

    #[test]
    fn encode_decode() {
        let from = secure::PublicKey::try_from_bytes(&random_vec(65)).unwrap();
        let to = secure::PublicKey::try_from_bytes(&random_vec(65)).unwrap();
        let data = random_vec(1024);

        let encoded = super::encode_unicast(from, to, data.clone());
        let (from_2, to_2, data_2) = super::decode_unicast(encoded).unwrap();

        assert_eq!(from, from_2);
        assert_eq!(to, to_2);
        assert_eq!(data, data_2);
    }

    fn random_vec(len: usize) -> Vec<u8> {
        let key = (0..len).map(|_| rand::random::<u8>()).collect::<Vec<_>>();
        key
    }
}
