use derive_more::{Display, FromStr};

use automorph::Automorph;

#[derive(Debug, Clone, PartialEq, Eq, Automorph)]
pub enum RoomSpace {
    Public,
    Private(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Automorph)]
pub enum MemberRole {
    Admin,
    Member,
    Viewer,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, FromStr, Display, Automorph)]
pub struct MemberId(String);

#[derive(Debug, Clone, PartialEq, Eq, Automorph)]
pub struct MemberName(String);

#[derive(Debug, Clone, PartialEq, Eq, Automorph)]
pub struct MemberInfo {
    pub id: MemberId,
    pub name: MemberName,
    pub role: MemberRole,
    pub space: RoomSpace,
}
