use std::io::{BufReader, Cursor, Read};
use std::path::PathBuf;
use axum::body::{to_bytes, Body, Bytes};
use axum::extract::{FromRequest, Multipart, Path, Request, State};
use axum::http::StatusCode;
use axum::Json;
use axum::response::IntoResponse;
use futures::TryFutureExt;
use tokio::fs;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader as TokioBufReader};
use tracing::info;
use uuid::Uuid;
use bytes::Bytes as BytesExt;
use chrono::{DateTime, Utc};
use common::{get_project_dev_path, get_temp_token_path};
use common::server_action::project_action::ProjectResponse;
use common::server_action::token_action::{FileInfo, TokenAction, UsedTokenActionResponse};
use crate::{AppState, ServerError, ServerResult};
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
            TokenAction::UploadFile { path } => {
                let path = match ensure_path_in_project_path(project_slug.clone(), &path, true, false).await{
                    Ok(path) => path,
                    Err(e) => {
                        return Json(UsedTokenActionResponse::Error(format!("Error: {}", e)))
                    }
                };
                 while let Ok(Some(field)) = form.next_field().await {
                    let bytes = field.bytes().await.unwrap_or_else(|_| Bytes::new());
                    return match tokio::fs::write(path, &bytes).await{
                        Ok(_) => Json(UsedTokenActionResponse::Ok),
                        Err(e) => {
                           Json(UsedTokenActionResponse::Error(format!("Error writing file: {}", e)))
                        }
                    }
                }
                Json(UsedTokenActionResponse::Error("No file uploaded".to_string()))
            }
            TokenAction::UploadDir { path } => {
                let path = match ensure_path_in_project_path(project_slug.clone(), &path, false, false).await{
                    Ok(path) => path,
                    Err(e) => {
                        return Json(UsedTokenActionResponse::Error(format!("Error: {}", e)))
                    }
                };
                while let Ok(Some(field)) = form.next_field().await {
                    let bytes = field.bytes().await.unwrap_or_else(|_| Bytes::new());
                    let cursor = Cursor::new(bytes);
                    let tokio_reader = BufReader::new(cursor);
                    let mut archive = zip::ZipArchive::new(tokio_reader).unwrap();
                    for i in 0..archive.len(){
                        let mut file = archive.by_index(i).unwrap();
                        let output_path = match file.enclosed_name() {
                            Some(file_path) => path.join(file_path),
                            None => continue,
                        };
                        if file.is_dir() {
                            println!("File {} extracted to \"{}\"", i, output_path.display());
                            fs::create_dir_all(&output_path).await.unwrap();
                        } else {
                            println!(
                                "File {} extracted to \"{}\" ({} bytes)",
                                i,
                                output_path.display(),
                                file.size()
                            );
                            if let Some(p) = output_path.parent() {
                                if !p.exists() {
                                    fs::create_dir_all(p).await.unwrap();
                                }
                            }
                            let mut outfile = std::fs::File::create(&output_path).unwrap();
                            std::io::copy(&mut file, &mut outfile).unwrap();
                        }

                    }
                    return Json(UsedTokenActionResponse::Ok);
                }
                Json(UsedTokenActionResponse::Error("No file uploaded".to_string()))
            }
            TokenAction::DownloadFile { path } => {
                let path = match ensure_path_in_project_path(project_slug.clone(), &path, true, true).await{
                    Ok(path) => path,
                    Err(e) => {
                        return Json(UsedTokenActionResponse::Error(format!("Error: {}", e)))
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
                let file = tokio::fs::File::open(path)
                    .await
                    .unwrap();
                let metadata = file.metadata().await.unwrap();
                let size = metadata.len();
                let modified = metadata.modified().unwrap();
                let modified: DateTime<Utc> = modified.into();
                let last_modified = modified.format("%a, %d %b %Y %T").to_string();
                let mut reader = TokioBufReader::new(file);
                let content = if size < 200_000{
                    let mut buf = Vec::new();
                    tokio::io::copy(&mut reader, &mut buf)
                        .await.unwrap();
                    Some(String::from_utf8(buf).unwrap())
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
                let path = match ensure_path_in_project_path(project_slug.clone(), &path, false, true).await{
                    Ok(path) => path,
                    Err(e) => {
                        return Json(UsedTokenActionResponse::Error(format!("Error: {}", e)))
                    }
                };                // Create a zip file
                Json(UsedTokenActionResponse::Ok)
            }
        }
    } else {
        info!("token action cache miss : {}", token);
        Json(UsedTokenActionResponse::Error("Token not found".to_string()))
    }
}

