pub trait WorkspaceService {
    async fn get(&self) -> Result<(), String>;
}
