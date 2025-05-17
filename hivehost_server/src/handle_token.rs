use crate::{AppState, ServerError};
use axum::body::Body;
use axum::extract::{Multipart, Path, State};
use axum::http::{header, Response, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use chrono::{DateTime, Utc};
use common::server_action::token_action::{
    FileInfo, FileUploadStatus, TokenAction, UsedTokenActionResponse,
};
use common::{ensure_path_in_project_path, get_project_dev_path};
use futures::StreamExt;
use tarpc::tokio_util::io::ReaderStream;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader as TokioBufReader;
use tracing::info;

pub async fn server_project_action_token(
    State(state): State<AppState>,
    Path(token): Path<String>,
    mut form: Multipart,
) -> impl IntoResponse {
    info!("server_project_action_token: {}", token);
    if let Some((project_slug, action)) = state.project_token_action_cache.remove(&token).await {
        info!(
            "token action cache hit : {:?} => {:?}",
            project_slug, action
        );
        match action {
            TokenAction::UploadFiles { path } => {
                let base_upload_path =
                    match ensure_path_in_project_path(&project_slug, &path, false, true).await {
                        Ok(path) => path,
                        Err(e) => {
                            return Json(UsedTokenActionResponse::Error(format!("Error: {e}")))
                                .into_response();
                        }
                    };
                let mut upload_statuses = Vec::new();
                while let Ok(Some(mut field)) = form.next_field().await {
                    // field needs to be mutable for .next()
                    let original_filename = match field.file_name() {
                        Some(name) => name.to_string(),
                        None => {
                            // Skip fields without filenames, or log/error as appropriate
                            continue;
                        }
                    };

                    // **Critical Security Step: Sanitize the filename**
                    // Use a crate like `sanitize-filename` or implement robust sanitization
                    // to prevent path traversal, illegal characters, etc.
                    let sanitized_filename = sanitize_filename::sanitize(&original_filename);
                    if sanitized_filename.is_empty()
                        || sanitized_filename.contains("..")
                        || sanitized_filename.contains("/")
                    {
                        upload_statuses.push(FileUploadStatus {
                            filename: original_filename,
                            success: false,
                            message: "Filename became empty after sanitization.".to_string(),
                        });
                        continue;
                    }

                    let final_path = base_upload_path.join(&sanitized_filename);
                    let final_path = match final_path.canonicalize() {
                        Ok(path) => path,
                        Err(e) => {
                            upload_statuses.push(FileUploadStatus {
                                filename: original_filename.clone(),
                                success: false,
                                message: format!("Error canonicalizing path: {e}"),
                            });
                            continue;
                        }
                    };

                    if !final_path.starts_with(&base_upload_path) {
                        upload_statuses.push(FileUploadStatus {
                            filename: original_filename.clone(),
                            success: false,
                            message: "Attempted path traversal detected.".to_string(),
                        });
                        continue;
                    }

                    match tokio::fs::File::create(&final_path).await {
                        Ok(mut file_to_write) => {
                            let mut field_successfully_streamed = true;
                            while let Some(chunk_result) = field.next().await {
                                match chunk_result {
                                    Ok(chunk) => {
                                        if let Err(e) = file_to_write.write_all(&chunk).await {
                                            upload_statuses.push(FileUploadStatus {
                                                filename: original_filename.clone(), // Use original for reporting
                                                success: false,
                                                message: format!(
                                                    "Error writing chunk to file: {e}"
                                                ),
                                            });
                                            field_successfully_streamed = false;
                                            // Attempt to clean up partially written file
                                            tokio::fs::remove_file(&final_path).await.ok();
                                            break; // Stop processing this field's chunks
                                        }
                                    }
                                    Err(e) => {
                                        upload_statuses.push(FileUploadStatus {
                                            filename: original_filename.clone(),
                                            success: false,
                                            message: format!(
                                                "Error reading chunk from upload stream: {e}"
                                            ),
                                        });
                                        field_successfully_streamed = false;
                                        // Attempt to clean up partially written file (if created)
                                        tokio::fs::remove_file(&final_path).await.ok();
                                        break; // Stop processing this field's chunks
                                    }
                                }
                            }

                            if field_successfully_streamed {
                                // Ensure data is flushed from OS buffers to disk
                                if let Err(e) = file_to_write.flush().await {
                                    upload_statuses.push(FileUploadStatus {
                                        filename: original_filename,
                                        success: false,
                                        message: format!(
                                            "Error flushing/syncing file to disk: {e}"
                                        ),
                                    });
                                    tokio::fs::remove_file(&final_path).await.ok();
                                // Clean up
                                } else {
                                    file_to_write.sync_all().await.unwrap();
                                    upload_statuses.push(FileUploadStatus {
                                        filename: original_filename,
                                        success: true,
                                        message: "Uploaded successfully.".to_string(),
                                    });
                                }
                            }
                        }
                        Err(e) => {
                            upload_statuses.push(FileUploadStatus {
                                filename: original_filename,
                                success: false,
                                message: format!("Error creating file '{sanitized_filename}': {e}"),
                            });
                        }
                    }
                }

                if upload_statuses.is_empty() && form.next_field().await.is_err() {
                    // Check if multipart itself had an error or was empty
                    Json(UsedTokenActionResponse::Error(
                        "No files were processed or multipart form was empty/invalid.".to_string(),
                    ))
                    .into_response()
                } else {
                    Json(UsedTokenActionResponse::UploadReport(upload_statuses)).into_response()
                }
            }
            TokenAction::ViewFile { path } => {
                let path = match ensure_path_in_project_path(&project_slug, &path, true, true).await
                {
                    Ok(path) => path,
                    Err(e) => {
                        return Json(UsedTokenActionResponse::Error(format!("Error: {e}")))
                            .into_response();
                    }
                };
                let path_copy = path.clone();
                let path_copy = path_copy
                    .strip_prefix(get_project_dev_path(&project_slug))
                    .unwrap();
                let name = path
                    .file_name()
                    .ok_or(ServerError::CantReadFileName(
                        path.to_string_lossy().to_string(),
                    ))
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                let file = File::open(path).await.unwrap();
                let metadata = file.metadata().await.unwrap();
                let size = metadata.len();
                let modified = metadata.modified().unwrap();
                let modified: DateTime<Utc> = modified.into();
                let last_modified = modified.format("%a, %d %b %Y %T").to_string();

                let content = if size < 64 * 1024 {
                    let mut reader = TokioBufReader::new(file);
                    let mut buf = Vec::new();
                    tokio::io::copy(&mut reader, &mut buf).await.unwrap();
                    Some(String::from_utf8_lossy(&buf).to_string())
                } else {
                    None
                };
                Json(UsedTokenActionResponse::File(FileInfo {
                    name,
                    content,
                    size,
                    path: format!("root/{}", path_copy.to_string_lossy()),
                    last_modified,
                }))
                .into_response()
            }
            TokenAction::UpdateFile { path } => {
                let target_path = ensure_path_in_project_path(&project_slug, &path, true, true)
                    .await
                    .unwrap();
                // Create a temporary file path. It's good practice to put it in the same filesystem/directory
                // to ensure atomic rename works and to inherit permissions/mount properties if relevant.
                let temp_file_name = format!(".tmp_update_{}", uuid::Uuid::new_v4());
                let temp_path = target_path.with_file_name(temp_file_name);

                // Expecting a single file field in the multipart form for the update content
                if let Ok(Some(mut field)) = form.next_field().await {
                    match tokio::fs::File::create(&temp_path).await {
                        Ok(mut temp_file_to_write) => {
                            while let Some(chunk_result) = field.next().await {
                                match chunk_result {
                                    Ok(chunk) => {
                                        if let Err(e) = temp_file_to_write.write_all(&chunk).await {
                                            // Log error
                                            tokio::fs::remove_file(&temp_path).await.ok(); // Clean up temp file
                                            return Json(UsedTokenActionResponse::Error(format!(
                                                "Error writing to temporary file: {e}"
                                            )))
                                            .into_response();
                                        }
                                    }
                                    Err(e) => {
                                        // Log error
                                        tokio::fs::remove_file(&temp_path).await.ok(); // Clean up temp file
                                        return Json(UsedTokenActionResponse::Error(format!(
                                            "Error reading update data chunk: {e}"
                                        )))
                                        .into_response();
                                    }
                                }
                            }

                            // Ensure data is flushed from OS buffers to disk before rename
                            if let Err(e) = temp_file_to_write.flush().await {
                                tokio::fs::remove_file(&temp_path).await.ok(); // Clean up temp file
                                return Json(UsedTokenActionResponse::Error(format!(
                                    "Error flushing/syncing temp file: {e}"
                                )))
                                .into_response();
                            } else {
                                temp_file_to_write.sync_all().await.unwrap();
                            }
                            // Explicitly drop/close temp_file_to_write before rename on some OSes, though tokio::fs::rename should handle it.
                            drop(temp_file_to_write);

                            // Atomically replace the old file with the new one
                            if let Err(e) = tokio::fs::rename(&temp_path, &target_path).await {
                                tokio::fs::remove_file(&temp_path).await.ok(); // Attempt to clean up temp file
                                Json(UsedTokenActionResponse::Error(format!(
                                    "Error finalizing file update (rename): {e}"
                                )))
                                .into_response()
                            } else {
                                Json(UsedTokenActionResponse::Ok).into_response()
                            }
                        }
                        Err(e) => Json(UsedTokenActionResponse::Error(format!(
                            "Error creating temporary file for update: {e}"
                        )))
                        .into_response(),
                    }
                } else {
                    // No file content provided, or error reading the first field
                    Json(UsedTokenActionResponse::Error(
                        "No content provided or error in multipart form for file update."
                            .to_string(),
                    ))
                    .into_response()
                }
            }
            TokenAction::DownloadFile { path } => {
                // Renamed to avoid conflict
                let validated_path = ensure_path_in_project_path(&project_slug, &path, true, true)
                    .await
                    .unwrap();

                let file = match tokio::fs::File::open(&validated_path).await {
                    Ok(f) => f,
                    Err(e) => {
                        // Log the error server-side for details
                        tracing::error!(
                            "Failed to open file for download {:?}: {}",
                            validated_path,
                            e
                        );
                        return Json(UsedTokenActionResponse::Error(
                            "File not found or could not be opened.".to_string(),
                        ))
                        .into_response();
                    }
                };

                // Get filename for Content-Disposition
                let filename = validated_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("downloaded_file") // Fallback filename
                    .to_string();

                // Convert the asynchronous file reader into a stream of byte chunks
                let stream = ReaderStream::new(file);

                // Create a response body from the stream
                let body = Body::from_stream(stream);

                // Build the HTTP response
                match Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/octet-stream") // Generic binary type
                    .header(
                        header::CONTENT_DISPOSITION,
                        format!(
                            "attachment; filename=\"{}\"",
                            sanitize_filename::sanitize(&filename)
                        ), // Sanitize filename for header
                    )
                    .body(body)
                {
                    Ok(response) => response.into_response(),
                    Err(e) => {
                        tracing::error!("Failed to build streaming response: {}", e);
                        Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::from("Internal server error"))
                            .unwrap()
                            .into_response()
                    }
                }
            }
        }
    } else {
        info!("token action cache miss : {}", token);
        Json(UsedTokenActionResponse::Error(
            "Token not found".to_string(),
        ))
        .into_response()
    }
}
