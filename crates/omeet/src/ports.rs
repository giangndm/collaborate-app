use futures::Stream;

pub trait EventRecorderPort {
    type Channel;
    type Event;
    fn append(&self, channel: &Self::Channel, event: Self::Event);
    fn subscribe(&self, channel: &Self::Channel) -> impl Stream<Item = Self::Event>;
    async fn restore(&self, channel: &Self::Channel) -> impl Iterator<Item = Self::Event>;
}
