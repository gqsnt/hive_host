use crate::api::get_action_server_project_action;
use common::server_project_action::io_action::file_action::FileAction;
use common::server_project_action::ServerProjectActionResponse;
use common::{ProjectSlugStr, StringContent};
use leptos::either::Either;
use leptos::prelude::{event_target_value, signal, Effect, ElementChild, GlobalAttributes, NodeRef, NodeRefAttribute, Read, Show};
use leptos::prelude::{ClassAttribute, Get, Resource, Signal, Transition};
use leptos::prelude::{GetUntracked, OnAttribute, OnTargetAttribute};
use leptos::prelude::{IntoMaybeErased, PropAttribute, RwSignal, ServerFnError, Set, Suspend};
use leptos::{component, view, IntoView};
use leptos::html::{Input, Textarea};
use leptos::logging::log;
use web_sys::MouseEvent;

#[component]
pub fn FileContentView(
    selected_file: Signal<Option<String>>,
    slug: Signal<ProjectSlugStr>,
) -> impl IntoView {
    
    let file_content_resource = Resource::new(
        move || (selected_file.get(), slug.get()),
        |(file_path_opt, slug)| async move {

            match file_path_opt {
                Some(file_path) => {
                    match crate::api::get_action_server_project_action_inner(
                        slug,
                        FileAction::View { path: file_path }.into(),
                        Default::default(),
                    )
                        .await
                    {
                        Ok(ServerProjectActionResponse::File(file_info)) => {
                            Ok(file_info)
                        },
                        Err(e) => {
                            leptos::logging::error!("Error fetching file: {:?}", e);
                            Err(ServerFnError::new("Failed to fetch file"))
                        },
                        _ => Err(ServerFnError::new("Invalid response type")),
                    }
                }
                _ => Err(ServerFnError::new("No file selected")),
            }
        },
    );





    view! {
        <Transition fallback=move || {
            view! { <p class="text-gray-400">"Loading file content..."</p> }
        }>
            {move || Suspend::new(async move {
                match selected_file.get() {
                    None => {
                        Either::Right(
                            view! {
                                <div class="flex items-center justify-center h-full text-gray-500">
                                    <p>"Select a file from the sidebar to view its content."</p>
                                </div>
                            },
                        )
                    }
                    Some(_) => {
                        Either::Left(
                            file_content_resource
                                .get()
                                .map(|result| {
                                    match result {
                                        Ok(file_info) => {
                                                let server_project_action = get_action_server_project_action();
                                            let (content_signal, set_content_signal) = signal(file_info.content.clone());
                                             let node_ref:NodeRef<Textarea>=  NodeRef::new();
                                            let path_clone = file_info.path.clone(); // Clone path for the closure
                                            // let handle_on_input = move |ev: web_sys::Event| {
                                            //     let target = ev.target().unwrap();
                                            //     let value = event_target_value(&target);
                                            //     // Update the signal with the new value
                                            //     set_content_signal(value);
                                            // };
                                        
                                            let on_click_update = move |ev: MouseEvent| {
                                                ev.prevent_default();
                                                //ev.prevent_default();
                                                // Use the editing_content signal for the update payload
                                                let content_to_save = node_ref.get().map(|t|t.value()).unwrap_or_default();
                                                    //set_content_signal(content_to_save.clone()); 
                                                server_project_action.dispatch((
                                                        slug.get(), // Use get() for signals
                                                        FileAction::Update {
                                                            path: path_clone.clone()
                                                        }.into(),
                                                        Some(content_to_save)
                                                    ));
                                            };
                                        
                                            Either::Left(view! {
                                                <div class="flex flex-col h-full"> // Ensure parent flex col takes height
                                                    // --- Enhanced Header ---
                                                    <div class="flex flex-wrap justify-between items-center gap-x-4 gap-y-2 mb-4 pb-4 border-b border-white/10 flex-shrink-0">
                                                        // File Name (prominent)
                                                        <h3 class="text-lg font-semibold text-white truncate mr-auto" title=file_info.path.clone()> // Add full path on hover
                                                            {file_info.name}
                                                        </h3>

                                                        // Metadata Group
                                                        <div class="flex items-center gap-x-3 text-sm text-gray-400">
                                                            <span>
                                                                "Size: "
                                                                <span class="font-medium text-gray-300">{format_bytes(file_info.size)}</span>
                                                            </span>
                                                            <span>|</span>
                                                            <span>
                                                                "Modified: "
                                                                <span class="font-medium text-gray-300">{file_info.last_modified}</span>
                                                            </span>
                                                        </div>

                                                         <button
                                                                type="button"
                                                                on:click=on_click_update
                                                                class="btn-primary px-3 py-1 text-sm" // Adjusted padding/text size
                                                                disabled = move || server_project_action.pending().get()
                                                            >
                                                                 Save Changes
                                                            </button>
                                                    </div>

                                                    // --- Styled Textarea ---
                                                    <div class="flex-grow min-h-0"> // Allow textarea container to grow and handle overflow
                                                        <textarea
                                                            wrap="off" // Keep horizontal scroll for code-like content
                                                            rows="40"
                                                            node_ref=node_ref
                                                            name="file_content"
                                                            class="w-full h-full resize-none bg-gray-800 text-gray-200 border border-gray-700 rounded-md p-3 font-mono text-sm focus:ring-1 focus:ring-indigo-500 focus:border-indigo-500 outline-none"
                                                                   
                                            // Use the derived display_content signal
                                                            // Update the specific editing_content signal on input
                                                        >
                                                        {content_signal.get_untracked()}
                                                        </textarea>
                                                     </div>
                                                </div>
                                            })
                                        }
                                        Err(e) => {
                                            // Log the error for debugging
                                            Either::Right(
                                                view! {
                                                    // <p class="text-red-400">"Error loading file data. Please try again."</p>
                                                },
                                            )
                                        }
                                    }
                                }),
                        )
                    }
                }
            })}
        </Transition>
    }
}


fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let bytes_f64 = bytes as f64;

    if bytes_f64 >= GB {
        format!("{:.2} GB", bytes_f64 / GB)
    } else if bytes_f64 >= MB {
        format!("{:.2} MB", bytes_f64 / MB)
    } else if bytes_f64 >= KB {
        format!("{:.2} KB", bytes_f64 / KB)
    } else {
        format!("{} Bytes", bytes)
    }
}
