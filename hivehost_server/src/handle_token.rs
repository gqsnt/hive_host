
use axum::body::{Bytes};
use axum::extract::{Multipart, Path, State};
use axum::Json;
use axum::response::IntoResponse;
use tokio::fs::{File};
use tokio::io::{BufReader as TokioBufReader};
use tracing::info;
use chrono::{DateTime, Utc};
use common::{get_project_dev_path};
use common::server_action::token_action::{FileInfo, FileUploadStatus, TokenAction, UsedTokenActionResponse};
use crate::{AppState, ServerError};
use crate::project_action::ensure_path_in_project_path;




pub async fn server_project_action_token(
    State(state): State<AppState>,
    Path(token): Path<String>,
    mut form: Multipart
) -> impl IntoResponse {
    info!("server_project_action_token: {}", token);
    if let Some((project_slug, action)) = state.project_token_action_cache.get(&token).await {
        state.project_token_action_cache.invalidate(&token).await;
        info!("token action cache hit : {} => {:?}", project_slug, action);
        match action{
            TokenAction::UploadFiles { path } => {
                let base_upload_path = match ensure_path_in_project_path(project_slug.clone(), &path, false, true).await{
                    Ok(path) => path,
                    Err(e) => {
                        return Json(UsedTokenActionResponse::Error(format!("Error: {e}")))
                    }
                };
                let mut upload_statuses = Vec::new();
                while let Ok(Some(field)) = form.next_field().await {
                    let original_filename = match field.file_name() {
                        Some(name) => name.to_string(),
                        None => {
                            // This part might be a non-file form field, or an issue with the upload.
                            // Log or decide how to handle. For now, skip.
                            // It's important that the frontend sends files with proper names.
                            continue;
                        }
                    };

                    // Basic sanitization (you might want more robust logic)
                    // if sanitized_filename.is_empty() {
                    //     upload_statuses.push(FileUploadStatus {
                    //         filename: original_filename,
                    //         success: false,
                    //         message: "Filename became empty after sanitization.".to_string(),
                    //     });
                    //     continue;
                    // }

                    let final_path = base_upload_path.join(&original_filename);

                    // Security: Ensure final_path is still within the project and intended directory.
                    // ensure_path_in_project_path could be used here again if it checks containment.
                    // For now, assuming base_upload_path + sanitized_filename is safe.

                    match field.bytes().await { // Reads entire file into memory. For very large files, stream to disk.
                        Ok(bytes) => {
                            match tokio::fs::write(&final_path, &bytes).await {
                                Ok(_) => {
                                    upload_statuses.push(FileUploadStatus {
                                        filename: original_filename.clone(),
                                        success: true,
                                        message: "Uploaded successfully.".to_string(),
                                    });
                                }
                                Err(e) => {
                                    upload_statuses.push(FileUploadStatus {
                                        filename: original_filename.clone(),
                                        success: false,
                                        message: format!("Error writing file: {e}"),
                                    });
                                }
                            }
                        }
                        Err(e) => {
                            upload_statuses.push(FileUploadStatus {
                                filename: original_filename.clone(),
                                success: false,
                                message: format!("Error reading file bytes: {e}"),
                            });
                        }
                    }
                }

                if upload_statuses.is_empty() {
                    Json(UsedTokenActionResponse::Error("No files were processed or found in the upload.".to_string()))
                } else {
                    Json(UsedTokenActionResponse::UploadReport(upload_statuses))
                }
            }
            TokenAction::ViewFile { path } => {
                let path = match ensure_path_in_project_path(project_slug.clone(), &path, true, true).await{
                    Ok(path) => path,
                    Err(e) => {
                        return Json(UsedTokenActionResponse::Error(format!("Error: {e}")))
                    }
                };                let path_copy = path.clone();
                let path_copy = path_copy
                    .strip_prefix(get_project_dev_path(&project_slug.to_string()))
                    .unwrap();
                let name = path
                    .file_name()
                    .ok_or(ServerError::CantReadFileName(
                        path.to_string_lossy().to_string(),
                    )).unwrap()
                    .to_string_lossy()
                    .to_string();
                let file = File::open(path)
                    .await
                    .unwrap();
                let metadata = file.metadata().await.unwrap();
                let size = metadata.len();
                let modified = metadata.modified().unwrap();
                let modified: DateTime<Utc> = modified.into();
                let last_modified = modified.format("%a, %d %b %Y %T").to_string();
    
                let content = if size < 200_000{
                    let mut reader = TokioBufReader::new(file);
                    let mut buf = Vec::new();
                    tokio::io::copy(&mut reader, &mut buf)
                        .await.unwrap();
                    Some(String::from_utf8_lossy(&buf).to_string())
                }else{
                    None
                };
                Json(UsedTokenActionResponse::File(FileInfo {
                    name,
                    content,
                    size,
                    path: format!("root/{}", path_copy.to_string_lossy()),
                    last_modified,
                }))
            }
            TokenAction::DownloadDir { path } => {
                let _path = match ensure_path_in_project_path(project_slug.clone(), &path, false, true).await{
                    Ok(path) => path,
                    Err(e) => {
                        return Json(UsedTokenActionResponse::Error(format!("Error: {e}")))
                    }
                };                // Create a zip file
                Json(UsedTokenActionResponse::Ok)
            }
            TokenAction::UpdateFile { path } => {
                let path = ensure_path_in_project_path(project_slug.clone(), &path, true, true).await.unwrap();
                while let Ok(Some(field)) = form.next_field().await {
                    let bytes = field.bytes().await.unwrap_or_else(|_| Bytes::new());
                    tokio::fs::write(path.clone(), &bytes).await.unwrap();
                }
                Json(UsedTokenActionResponse::Ok)
            }
            TokenAction::DownloadFile { path } => {
                let path = match ensure_path_in_project_path(project_slug.clone(), &path, true, true).await{
                    Ok(path) => path,
                    Err(e) => {
                        return Json(UsedTokenActionResponse::Error(format!("Error: {e}")))
                    }
                };
                let file = File::open(path)
                    .await
                    .unwrap();
                let mut reader = TokioBufReader::new(file);
                let mut buf = Vec::new();
                tokio::io::copy(&mut reader, &mut buf)
                    .await.unwrap();
                Json(UsedTokenActionResponse::Content(buf))
                
                
            }
        }
    } else {
        info!("token action cache miss : {}", token);
        Json(UsedTokenActionResponse::Error("Token not found".to_string()))
    }
}

