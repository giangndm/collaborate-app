use futures::Stream;

use crate::domain::room::types::RoomId;

pub trait RoomService {
    async fn join_room(&self, room_id: &RoomId);
    async fn leave_room(&self, room_id: &RoomId);
    async fn room_mutation(&self, room_id: &RoomId, rpc: &str);
    async fn room_subscribe(
        &self,
        room_id: &RoomId,
        channel: &str,
    ) -> Result<impl Stream<Item = String>, String>;
}

pub struct RoomServiceImpl {}

impl RoomService for RoomServiceImpl {
    async fn join_room(&self, room_id: &RoomId) {
        todo!()
    }

    async fn leave_room(&self, room_id: &RoomId) {
        todo!()
    }

    async fn room_mutation(&self, room_id: &RoomId, rpc: &str) {
        todo!()
    }

    async fn room_subscribe(
        &self,
        room_id: &RoomId,
        channel: &str,
    ) -> Result<impl Stream<Item = String>, String> {
        todo!()
    }
}
