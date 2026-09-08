//! The file RPC dispatch, including project-scoped mutations.
use super::*;
use serde_json::Value;

impl EngineRpc {
    pub(super) async fn handle_files(
        &self,
        method: &str,
        params: Value,
    ) -> Result<RpcReply, RpcError> {
        match method {
            methods::FILES_TREE => {
                let p: surya_proto::files::FileTreeParams = parse_params(params)?;
                let jail = self.files_jail(&p.space_id).await?;
                let tree = tokio::task::spawn_blocking(move || {
                    crate::files::tree(&jail, &p.path, p.depth)
                })
                .await
                .map_err(|e| RpcError::Failed(e.to_string()))?
                .map_err(|e| RpcError::Failed(e.to_string()))?;
                RpcReply::value(&tree)
            }
            methods::FILES_WATCH => {
                let p: surya_proto::files::FileWatchParams = parse_params(params)?;
                let jail = self.files_jail(&p.space_id).await?;
                // Recursive inotify adds and the gitignore reads are sync work;
                // build the watcher on the blocking pool like diff_sync does.
                let batches = tokio::task::spawn_blocking(move || crate::files_watch::watch(jail))
                    .await
                    .map_err(|e| RpcError::Failed(e.to_string()))?
                    .map_err(|e| RpcError::Failed(e.to_string()))?;
                Ok(RpcReply::Stream(
                    batches
                        .filter_map(|batch| async move { serde_json::to_value(&batch).ok() })
                        .boxed(),
                ))
            }
            methods::FILES_READ => {
                let p: surya_proto::files::FileReadParams = parse_params(params)?;
                let jail = self.files_jail(&p.space_id).await?;
                let read = tokio::task::spawn_blocking(move || {
                    crate::files::read(&jail, &p.path, p.range)
                })
                .await
                .map_err(|e| RpcError::Failed(e.to_string()))?
                .map_err(|e| RpcError::Failed(e.to_string()))?;
                RpcReply::value(&read)
            }
            methods::FILES_WRITE => {
                let p: surya_proto::files::FileWriteParams = parse_params(params)?;
                let jail = self.files_jail(&p.space_id).await?;
                let written = tokio::task::spawn_blocking(move || {
                    crate::files::write(&jail, &p.path, &p.content, p.expected_hash.as_deref())
                })
                .await
                .map_err(|e| RpcError::Failed(e.to_string()))?
                .map_err(|e| RpcError::Failed(e.to_string()))?;
                RpcReply::value(&written)
            }
            methods::FILES_SEARCH => {
                let p: surya_proto::files::FileNameSearchParams = parse_params(params)?;
                if p.query.chars().count() > 256 {
                    return Err(RpcError::BadParams(
                        "FilesSearch query must not exceed 256 characters".into(),
                    ));
                }
                let jail = self.files_jail(&p.space_id).await?;
                let matches = tokio::time::timeout(
                    FILE_SEARCH_RPC_TIMEOUT,
                    self.repos
                        .search_files(jail.root().to_path_buf(), p.query, Vec::new()),
                )
                .await
                .map_err(|_| RpcError::Failed("file search timed out".into()))?
                .map_err(|e| RpcError::Failed(e.to_string()))?;
                RpcReply::value(&matches)
            }
            methods::FILES_MUTATE => {
                let p: surya_proto::files::FileMutationParams = parse_params(params)?;
                let jail = self.files_jail(&p.space_id).await?;
                tokio::task::spawn_blocking(move || {
                    crate::files::mutations::mutate(&jail, &p.action)
                })
                .await
                .map_err(|e| RpcError::Failed(e.to_string()))?
                .map_err(|e| RpcError::Failed(e.to_string()))?;
                RpcReply::value(&serde_json::json!({ "changed": true }))
            }
            _ => Err(RpcError::UnknownMethod(method.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_actions_are_forwarded_to_the_projects_device() {
        assert!(forwardable(methods::FILES_MUTATE));
        assert!(!is_stream_method(methods::FILES_MUTATE));
    }
}
