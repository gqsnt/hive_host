use crate::api::{get_action_server_project_action, get_action_token_action};
use common::server_action::permission::Permission;
use common::server_action::project_action::io_action::file_action::ProjectIoFileAction;
use common::server_action::project_action::ProjectResponse;
use common::ProjectSlugStr;
use leptos::either::{Either, EitherOf4, EitherOf5};
use leptos::html::{Input, Textarea};
use leptos::prelude::{event_target_value, signal, Effect, ElementChild, For, GlobalAttributes, NodeRef, NodeRefAttribute, PropAttribute, Read, Show};
use leptos::prelude::{ClassAttribute, Get, Resource, Signal, Transition};
use leptos::prelude::{GetUntracked, OnAttribute};
use leptos::prelude::{IntoMaybeErased, ServerFnError, Suspend};
use leptos::{component, view, IntoView};
use leptos::leptos_dom::log;
use leptos::reactive::spawn_local;
use leptos::server::LocalResource;
use wasm_bindgen::JsCast;
use web_sys::{FormData, HtmlFormElement, MouseEvent, SubmitEvent};
use common::server_action::token_action::{FileInfo, TokenAction, UsedTokenActionResponse};

#[component]
pub fn FileContentView(
    selected_file: Signal<Option<String>>,
    slug: Signal<ProjectSlugStr>,
    csrf_signal: Signal<Option<String>>, // Assuming this provides the CSRF token string
    permission_signal: Signal<Permission>,
) -> impl IntoView {
    // Resource to fetch file content
    let file_content_resource = LocalResource::new(
        move || async move {
            match selected_file.get() {
                Some(file_path) => {
                    match crate::api::get_action_token_action(
                        slug.get(),
                        TokenAction::DownloadFile { path: file_path }.into(),
                        None,
                        None,
                    )
                        .await
                    {
                        Ok(UsedTokenActionResponse::File(file_info)) => Ok(file_info),
                        Err(e) => {
                            leptos::logging::error!("Error fetching file: {:?}", e);
                            Err(ServerFnError::new("Failed to fetch file"))
                        }
                        _ => Err(ServerFnError::new("Invalid response type")),
                    }
                }
                _ => Err(ServerFnError::new("FileContentView: No file selected")),
            }
        },
    );

    let (current_file_path_for_form, set_current_file_path_for_form) = signal(String::new());

    // Effect to reset editing state when selected file changes or content loads

    
    let server_save_action = get_action_server_project_action(); // Assuming this is a general action for project ops
    let node_ref: NodeRef<Textarea> = NodeRef::new();
    


    let handle_on_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        
        let form_data = FormData::new().unwrap(); // Create new FormData
        // Append the content from the signal, not just relying on form field
        form_data.append_with_str("file_content", &node_ref.get().unwrap().value()).unwrap();

        let path_to_save = current_file_path_for_form.get_untracked();
        let csrf_token_val = csrf_signal.get_untracked(); // Get current value

        spawn_local(async move {
            match get_action_token_action(
                slug.get(), // Or slug.get_untracked() if appropriate
                TokenAction::UpdateFile { path: path_to_save }.into(),
                csrf_token_val,
                Some(form_data),
            )
                .await
            {
                Ok(UsedTokenActionResponse::Ok) => {
                    // Successfully saved
                    // Optionally, re-fetch or update content display if server modifies it
                    // For now, just clear dirty flag. User sees their own changes.
                    log!("File saved successfully.");
                    // file_content_resource.refetch(); // To get fresh last_modified, etc.
                }
                Ok(UsedTokenActionResponse::Error(e)) => {
                    leptos::logging::error!("Error saving file: {:?}", e);
                    // Show error to user
                }
                Err(e) => {
                    leptos::logging::error!("Server error saving file: {:?}", e);
                    // Show error to user
                }
                _ => {
                    leptos::logging::warn!("Unexpected response type after saving file.");
                }
            }
        });
    };


    view! {
        <Transition fallback=move || view! { <p class="text-gray-400">"Loading..."</p> }>
               {move || Suspend::new(async move { match (selected_file.get(), file_content_resource.get()) {
                (None, _) => EitherOf4::A(view! {
                    <div class="flex items-center justify-center h-full text-gray-500">
                        <p>"Select a file from the sidebar to view its content."</p>
                    </div>
                }),
                (Some(_), None) => EitherOf4::B(view! { // Still loading resource
                    <p class="text-gray-400">"Fetching file details..."</p>
                }),
                (Some(_), Some(Err(e))) => EitherOf4::C(view! {
                    <p class="text-red-400">{format!("Error loading file: {:?}", e)}</p>
                }),
                (Some(_), Some(Ok(file_info))) => {
                    let can_edit = permission_signal.get().can_edit();
                       set_current_file_path_for_form(file_info.path.clone());
                       let content = file_info.content.clone();
                       let has_content  = content.is_some();   
                       let content = Signal::derive(move || content.clone().unwrap_or_default());
                    EitherOf4::D(view! {
                        <div class="flex flex-col h-full">
                            // --- Enhanced Header ---
                            <div class="flex flex-wrap justify-between items-center gap-x-4 gap-y-2 mb-4 pb-4 border-b border-white/10 flex-shrink-0">
                                <h3
                                    class="text-lg font-semibold text-white truncate mr-auto"
                                    title=file_info.path.clone()
                                >
                                    {file_info.name.clone()} 
                                    //{if can_edit { "*" } else { "" }}
                                </h3>
                                <div class="flex items-center gap-x-3 text-sm text-gray-400">
                                    <span>"Size: " <span class="font-medium text-gray-300">{format_bytes(file_info.size)}</span></span>
                                    <span>"|"</span>
                                    <span>"Modified: " <span class="font-medium text-gray-300">{file_info.last_modified.clone()}</span></span>
                                </div>
                                <Show when=move || can_edit && has_content>
                                    <form on:submit=handle_on_submit class="contents"> // Use "contents" to not break flex layout
                                        // Hidden input to carry path if needed by form logic, though we use signal
                                        // <input type="hidden" name="file_path" value={file_info.path.clone()} />
                                        <button
                                            type="submit"
                                            class="btn btn-primary px-3 py-1 text-sm"
                                            disabled=move || server_save_action.pending().get()
                                        >
                                            Save Changes
                                        </button>
                                    </form>
                                </Show>
                            </div>
                           <Show when=move || has_content  fallback=move ||  view!{
                              <div class="flex-grow p-4 text-gray-400">
                                            <p class="font-semibold mb-2">"Content not displayable in editor."</p>
                                            <p>"This might be a binary file, too large, or not valid text."</p>
                                            // Add a Download button here if desired
                                            // Example:
                                            // <button class="btn btn-secondary mt-4" on:click=move |_| { /* trigger download action */ }>
                                            // Download File
                                            // </button>
                                        </div>  
                           }>
                            <div class="flex-grow min-h-0"> // Container for textarea
                                            <textarea
                                                wrap="off"
                                                rows="40" // Initial hint, but flex styling should control height
                                                name="file_content" // Still useful if form submission relies on it directly elsewhere
                                                class="w-full h-full resize-none bg-gray-800 text-gray-200 border border-gray-700 rounded-md p-3 font-mono text-sm focus:ring-1 focus:ring-indigo-500 focus:border-indigo-500 outline-none"
                                                node_ref=node_ref
                                                //on:input=move |ev|{ set_is_dirty(true)}
                                                readonly={!can_edit}
                                                disabled={!can_edit} // `disabled` also makes it read-only visually
                                            >
                                                {content.get_untracked()}
                                            </textarea>
                                        </div>
                           </Show>
                        </div>
                    })
                }
            }})}
        </Transition>
    }
}




pub mod server_fns {
    cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
    }}
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
        format!("{bytes} Bytes")
    }
}
