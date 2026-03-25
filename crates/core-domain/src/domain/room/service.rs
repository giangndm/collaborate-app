#![allow(dead_code)]
use futures::{Stream, stream};

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
    async fn join_room(&self, _room_id: &RoomId) {
        todo!()
    }

    async fn leave_room(&self, _room_id: &RoomId) {
        todo!()
    }

    async fn room_mutation(&self, _room_id: &RoomId, _rpc: &str) {
        todo!()
    }

    async fn room_subscribe(
        &self,
        _room_id: &RoomId,
        _channel: &str,
    ) -> Result<impl Stream<Item = String>, String> {
        Ok::<stream::Empty<String>, String>(stream::empty())
    }
}
