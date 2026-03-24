use std::str::FromStr as StdFromStr;

use derive_more::{Display, FromStr};
use syncable_state::{PathSegment, SyncPath, SyncableState, SyncableString};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomSpace {
    Public,
    Private(String),
}

impl std::fmt::Display for RoomSpace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoomSpace::Public => write!(f, "Public"),
            RoomSpace::Private(s) => write!(f, "Private:{}", s),
        }
    }
}

impl StdFromStr for RoomSpace {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "Public" {
            Ok(RoomSpace::Public)
        } else if let Some(stripped) = s.strip_prefix("Private:") {
            Ok(RoomSpace::Private(stripped.to_string()))
        } else {
            Err(())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberRole {
    Admin,
    Member,
    Viewer,
}

impl std::fmt::Display for MemberRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemberRole::Admin => write!(f, "Admin"),
            MemberRole::Member => write!(f, "Member"),
            MemberRole::Viewer => write!(f, "Viewer"),
        }
    }
}

impl StdFromStr for MemberRole {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Admin" => Ok(MemberRole::Admin),
            "Member" => Ok(MemberRole::Member),
            "Viewer" => Ok(MemberRole::Viewer),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, FromStr, Display, Ord, PartialOrd)]
pub struct MemberId(pub String);

impl MemberId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberName(pub String);

#[derive(Debug, Clone, SyncableState)]
pub struct MemberInfo {
    #[sync(id)]
    pub id: MemberId,
    pub name: SyncableString,
    pub role: SyncableString,
    pub space: SyncableString,
}

impl MemberInfo {
    pub fn new(
        root_path: &SyncPath,
        id: MemberId,
        name: MemberName,
        role: MemberRole,
        space: RoomSpace,
    ) -> Self {
        let mut path_name = root_path.clone().into_vec();
        path_name.push(PathSegment::Field("name".into()));

        let mut path_role = root_path.clone().into_vec();
        path_role.push(PathSegment::Field("role".into()));

        let mut path_space = root_path.clone().into_vec();
        path_space.push(PathSegment::Field("space".into()));

        Self {
            id,
            name: SyncableString::new(SyncPath::new(path_name), name.0),
            role: SyncableString::new(SyncPath::new(path_role), role.to_string()),
            space: SyncableString::new(SyncPath::new(path_space), space.to_string()),
        }
    }

    pub fn get_id(&self) -> MemberId {
        self.id.clone()
    }

    pub fn get_name(&self) -> MemberName {
        MemberName(self.name.value().to_string())
    }

    pub fn get_role(&self) -> MemberRole {
        MemberRole::from_str(self.role.value()).unwrap_or(MemberRole::Viewer)
    }

    pub fn get_space(&self) -> RoomSpace {
        RoomSpace::from_str(self.space.value()).unwrap_or(RoomSpace::Public)
    }
}
