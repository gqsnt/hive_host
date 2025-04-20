use leptos::prelude::IntoAnyAttribute;
use leptos::prelude::AddAnyAttr;
use leptos::prelude::{event_target_value, ActionForm, CustomAttribute, GlobalAttributes, PropAttribute, Show};
use std::str::FromStr;
use crate::app::ServerFnError;
use common::server_project_action::io_action::dir_action::{DirAction, LsElement};
use leptos::prelude::{expect_context, signal, ClassAttribute, CollectView, Effect, For, Get, IntoMaybeErased, OnAttribute, RwSignal, ServerAction, Set, Suspend, Suspense};
use leptos::prelude::ElementChild;
use leptos::{component, server, view};
use leptos::either::{Either, EitherOf3, EitherOf4};
use leptos::logging::log;
use leptos::server::Resource;
use web_sys::SubmitEvent;
use common::{ProjectId, ProjectSlug, ProjectSlugStr};
use common::permission::Permission;
use common::server_project_action::io_action::file_action::FileInfo;
use common::server_project_action::permission::PermissionAction;
use common::server_project_action::ServerProjectActionResponse;

use crate::app::IntoView;
use crate::app::pages::user::projects::project::MemoProjectParams;
use crate::BoolInput;
use crate::security::permission::PermissionResult;

#[component]
pub fn ProjectFiles() -> impl IntoView {
    let params: MemoProjectParams = expect_context();
    let slug = move || params.get().unwrap().project_slug.clone();
    let (current_path, set_current_path) = signal(".".to_string());
    let create_folder_action = ServerAction::<CreateFolder>::new();
    let rename_item_action = ServerAction::<RenameItem>::new();
    let delete_item_action = ServerAction::<DeleteItem>::new();


    // Reload resource when reload_signal changes
    let ls_files = Resource::new(
        move || (
            current_path.get(),
            slug(),
            create_folder_action.version().get(),
        rename_item_action.version().get(),
        delete_item_action.version().get(),
        ), // Depend on trigger
        move |(path, slug_result, _,_,_)| {
            get_list_file(slug_result, path)
        }
    );

    // -- Navigation Helpers --
    let navigate_to = move |new_path: String| {
        // Basic path joining (consider edge cases like root "//")
        let resolved_path = if new_path == "." {
            ".".to_string()
        } else {
            format!("./{}", new_path.trim_start_matches('/'))
        };
        set_current_path(resolved_path);
    };

    let go_up_one_level = move |_| {
        let current = current_path.get(); // e.g., "./dir1/dir11" or "./dir1" or "."
        if current != "." {
            if let Some(last_slash_idx) = current.rfind('/') {
                // Check if the last slash is the one directly after the leading '.' (index 1)
                if last_slash_idx == 1 { // Path is like "./segment"
                    set_current_path(".".to_string()); // Parent is the root "."
                } else { // Path is like "./segment1/segment2"
                    set_current_path(current[..last_slash_idx].to_string()); // Parent is "./segment1"
                }
            }
            // else case: Path is malformed if it's not "." but doesn't contain '/' after index 0
        }
    };

    // Helper to build breadcrumbs
    let breadcrumbs = move || {
        let path = current_path.get(); // e.g., ".", "./dir1", "./dir1/dir11"
        // Start with the root segment
        let mut segments = vec![("Root".to_string(), ".".to_string())]; // Display "Root", Target is "."

        // Process non-root paths
        if path != "." {
            let mut accumulated_path = String::from("."); // Start accumulation from root "."

            // Iterate over segments after the leading "./"
            let relative_path = path.trim_start_matches("./"); // e.g., "dir1" or "dir1/dir11"
            for segment in relative_path.split('/') {
                if !segment.is_empty() {
                    // Build the target path for this segment (e.g., "./dir1", "./dir1/dir11")
                    accumulated_path.push('/');
                    accumulated_path.push_str(segment);

                    // Add the segment display name and its corresponding target path
                    segments.push((segment.to_string(), accumulated_path.clone()));
                }
            }
        }
        segments
    };


    // State for showing rename input
    // Tuple: (Option<(item_name, is_dir)>, Option<new_name_signal>)
    let (renaming_item, set_renaming_item) = signal::<Option<(String, bool)>>(None);
    let (new_name_input, set_new_name_input) = signal("".to_string());


    view! {
        <div class="flex flex-col h-full"> // Occupy available height
            // --- Header: Breadcrumbs and Actions ---
            <div class="flex items-center justify-between p-4 border-b border-white/10 mb-4">
                // Breadcrumbs
               <nav class="flex items-center space-x-1 text-sm text-gray-400 flex-wrap"> // flex-wrap for smaller screens
                    {move || breadcrumbs().into_iter().enumerate().map(|(i, (name, target_path))| {
                        let is_last = i == breadcrumbs().len() - 1;
                        view! {
                            <span class="flex items-center">
                                // Separator for non-root segments
                                {(i > 0).then(||view!{<span class="mx-1"> / </span>} ) }

                                // Segment Name (Clickable Button or Static Text)
                                {if is_last {
                                    // Last segment: Display name, not clickable
                                    Either::Left(view! { <span class="font-medium text-white whitespace-nowrap">{name}</span> })
                                } else {
                                    // Non-last segment: Display name as clickable button
                                    Either::Right(view! {
                                        <button
                                            class="hover:text-white hover:underline whitespace-nowrap"
                                            on:click=move |_| set_current_path(target_path.clone()) // Use set_current_path directly
                                        >
                                            {name}
                                        </button>
                                    })
                                }}
                            </span>
                        }
                    }).collect_view()}
                </nav>

                // Action Buttons (Example: Add Folder)
                <div class="flex items-center gap-x-2 flex-shrink-0">
                     // TODO: Add Folder Button/Form (maybe modal)
                    <button class="btn-primary" on:click=move |_| {
                        // Placeholder: Use prompt or a modal
                        if let Some(window) = web_sys::window() {
                            if let Ok(Some(name)) = window.prompt_with_message("Enter new folder name:") {
                                if !name.trim().is_empty() {
                                    create_folder_action.dispatch(CreateFolder {
                                            project_slug: slug(),
                                            path: current_path.get(),
                                            folder_name: name,
                                        });
                                }
                            }
                        }
                    }>
                        // Add SVG Icon?
                        "New Folder"
                    </button>
                     // TODO: Add Upload File Button
                </div>
            </div>

            // --- File Listing Area ---
            <div class="flex-grow overflow-y-auto p-4"> // Allow scrolling
                <Suspense fallback=move || view! { <p class="text-gray-400">"Loading files..."</p> }>
                    {move || {
                        ls_files.get().map(|result| match result {
                            Ok(list) => {
                                 if list.is_empty() && current_path.get() == "." {
                                    // Special view for empty root
                                     EitherOf4::A(view! {
                                        <div class="text-center text-gray-500 mt-10">
                                            <p>"Project is empty."</p>
                                        </div>
                                    })
                                } else if list.is_empty() {
                                     EitherOf4::B(view! {
                                        <div class="text-center text-gray-500 mt-10">
                                            // Optional: Add Icon
                                            <p>"This folder is empty."</p>
                                            <button class="mt-4 text-indigo-400 hover:text-indigo-300" on:click=go_up_one_level>
                                                "Go Up"
                                            </button>
                                        </div>
                                    })
                                } else {
                                    EitherOf4::C(view! {
                                        <table class="table">
                                            <tbody class="divide-y divide-gray-800">
                                                // Add '..' entry for going up, if not at root - FIXED Check
                                                {(current_path.get() != ".").then(|| view!{
                                                        <tr class="hover:bg-white/5">
                                                            <td class="table-td">
                                                                <button class="flex items-center gap-x-2 text-indigo-400 hover:text-indigo-300 w-full" on:click=go_up_one_level>
                                                                     <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
                                                                        <path stroke-linecap="round" stroke-linejoin="round" d="M9 9l6-6m0 0l6 6m-6-6v12a6 6 0 01-12 0v-3" />
                                                                    </svg>
                                                                    ".."
                                                                </button>
                                                            </td>
                                                            <td class="table-td"></td> // Empty actions cell for '..'
                                                        </tr>
                                                    })}


                                                // File and Folder List
                                                <For
                                                    each=move || list.clone()
                                                    key=|item| format!("{}-{}", item.name, item.is_dir)
                                                    children=move |item| {
                                                        // Use item directly where possible, avoid unnecessary signals if item doesn't change reactively within the row
                                                           let (item_name_s,_) = signal(item.name.clone());
                                                        let item_is_dir = item.is_dir;

                                                        let is_renaming_this = move || renaming_item.get().map_or(false, |(name, is_dir)| name == item_name_s() && is_dir == item_is_dir);

                                                        let start_rename = move |_| {
                                                            set_new_name_input(item_name_s());
                                                            set_renaming_item(Some((item_name_s(), item_is_dir)));
                                                        };
                                                        let cancel_rename = move |_| {
                                                            set_renaming_item(None);
                                                        };

                                                        let submit_rename = move |ev: SubmitEvent| {
                                                            ev.prevent_default(); // Always prevent default first
                                                            let mut allow_dispatch = false;
                                                            if let Some((old_name, _)) = renaming_item.get() {
                                                                let new_name = new_name_input.get();
                                                                if !new_name.trim().is_empty() && new_name != old_name {
                                                                     allow_dispatch = true;
                                                                     rename_item_action.dispatch(RenameItem {
                                                                            project_slug: slug(),
                                                                            path: current_path.get(),
                                                                            old_name: old_name.clone(),
                                                                            new_name: new_name.clone(),
                                                                        });
                                                                } else {
                                                                    log!("Rename cancelled: Name empty or unchanged.");
                                                                }
                                                            }
                                                            // Only close input if dispatch was attempted or name was invalid
                                                            set_renaming_item(None);
                                                        };


                                                        let submit_delete = move |ev: SubmitEvent| {
                                                            ev.prevent_default(); // Always prevent default
                                                            let confirmed = web_sys::window().map_or(false, |w|
                                                                 w.confirm_with_message(&format!("Are you sure you want to delete '{}'?", item_name_s())).unwrap_or(false)
                                                            );

                                                            if confirmed {
                                                               delete_item_action.dispatch(DeleteItem{
                                                                        project_slug: slug(),
                                                                        path: current_path.get(),
                                                                        name: item_name_s(),
                                                                        is_dir: BoolInput(item_is_dir), // Ensure BoolInput wraps correctly
                                                                    });
                                                            } else {
                                                                log!("Delete action cancelled by user.");
                                                            }
                                                        };
                                                     
                                                        view! {
                                                            <tr class="group hover:bg-white/5">
                                                                <td class="table-td py-2"> // Reduced padding maybe
                                                                    {move || if is_renaming_this() {
                                                                        // --- Rename Form ---

                                                                        Either::Left(view! {
                                                                             // Note: ActionForm itself doesn't need on:submit if using button type="submit"
                                                                             // on:submit should be on the <form> element generated by ActionForm
                                                                            <ActionForm action=rename_item_action attr:class="flex items-center gap-x-2" on:submit=submit_rename>
                                                                                <input type="hidden" name="project_slug" value=slug()/>
                                                                                <input type="hidden" name="path" value=current_path.get()/>
                                                                                <input type="hidden" name="old_name" value=item_name_s()/>
                                                                                <input
                                                                                    type="text"
                                                                                    name="new_name" // Name matches server fn arg
                                                                                    class="form-input flex-grow px-2 py-1 text-sm" // Adjusted input style
                                                                                    prop:value=new_name_input
                                                                                    on:input=move |ev| set_new_name_input(event_target_value(&ev))
                                                                                    // Consider JS focus: node_ref.get().expect("input exists").focus().ok();
                                                                                />
                                                                                <button type="submit" class="p-1 text-green-400 hover:text-green-300" title="Save">
                                                                                     <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5"><path stroke-linecap="round" stroke-linejoin="round" d="M4.5 12.75l6 6 9-13.5" /></svg>
                                                                                </button>
                                                                                <button type="button" on:click=cancel_rename class="p-1 text-gray-400 hover:text-white" title="Cancel">
                                                                                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5"><path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" /></svg>
                                                                                </button>
                                                                            </ActionForm>
                                                                        })
                                                                    } else {
                                                                        // --- Display Item ---
                                                                        Either::Right(view! {
                                                                            <div class="flex items-center gap-x-3">
                                                                                 // Icon
                                                                                {if item_is_dir {
                                                                                    Either::Left(view! { <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6 text-sky-400 flex-shrink-0"><path stroke-linecap="round" stroke-linejoin="round" d="M2.25 12.75V12A2.25 2.25 0 014.5 9.75h15A2.25 2.25 0 0121.75 12v.75m-8.69-6.44l-2.12-2.12a1.5 1.5 0 00-1.061-.44H4.5A2.25 2.25 0 002.25 6v12a2.25 2.25 0 002.25 2.25h15A2.25 2.25 0 0021.75 18V9a2.25 2.25 0 00-2.25-2.25h-5.379a1.5 1.5 0 01-1.06-.44z" /></svg> })
                                                                                } else {
                                                                                    Either::Right(view! { <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6 text-gray-400 flex-shrink-0"><path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m2.25 0H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9z" /></svg> })
                                                                                }}
                                                                                // Name (Clickable for folders)
                                                                                {if item_is_dir {
                                                                                   Either::Left(view! {
                                                                                        <button
                                                                                            class="text-left hover:text-white truncate"
                                                                                            on:click=move |_| {
                                                                                                let current = current_path.get();
                                                                                                // Construct path relative to internal representation
                                                                                                let next_path = if current == "." {
                                                                                                    format!("./{}", item_name_s())
                                                                                                } else {
                                                                                                    format!("{}/{}", current, item_name_s())
                                                                                                };
                                                                                                set_current_path(next_path);
                                                                                            }
                                                                                        >
                                                                                            {item_name_s()} // Display static name
                                                                                        </button>
                                                                                    })
                                                                                } else {
                                                                                    Either::Right(view! { <span class="truncate">{item_name_s()}</span> })
                                                                                }}
                                                                            </div>
                                                                        })
                                                                    }}
                                                                </td>
                                                                <td class="table-td py-2"> // Match padding
                                                                    // Action Buttons
                                                                    <div class="flex items-center justify-end gap-x-2 opacity-0 group-hover:opacity-100 focus-within:opacity-100 transition-opacity">
                                                                         <button on:click=start_rename class="p-1 text-gray-400 hover:text-white" title="Rename">
                                                                             <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4"><path stroke-linecap="round" stroke-linejoin="round" d="M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L6.832 19.82a4.5 4.5 0 01-1.897 1.13l-2.685.8.8-2.685a4.5 4.5 0 011.13-1.897L16.863 4.487zm0 0L19.5 7.125" /></svg>
                                                                         </button>
                                                                        <ActionForm action=delete_item_action on:submit=submit_delete>
                                                                             <input type="hidden" name="project_slug" value=slug()/>
                                                                             <input type="hidden" name="path" value=current_path.get()/>
                                                                             <input type="hidden" name="name" value=item_name_s()/>
                                                                             <input type="hidden" name="is_dir" value=item_is_dir.to_string()/> // Send bool as string for form
                                                                             <button type="submit" class="p-1 text-red-500 hover:text-red-400" title="Delete">
                                                                                 <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4"><path stroke-linecap="round" stroke-linejoin="round" d="M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0" /></svg>
                                                                             </button>
                                                                         </ActionForm>
                                                                    </div>
                                                                </td>
                                                            </tr>
                                                        }
                                                    }
                                                />
                                            </tbody>
                                        </table>
                                    })
                                }
                            },
                            Err(e) => {
                                 EitherOf4::D(view! { <p class="text-red-400">"Error loading files: " {e.to_string()}</p> })
                            }
                        }) // End map
                    }} // End Suspense children closure
                </Suspense>

                 // Display Action feedback (optional)
                <div class="mt-4 text-sm">
                    <Show when=move || create_folder_action.pending().get()> <p class="text-gray-400">"Creating folder..."</p> </Show>
                    <Show when=move || rename_item_action.pending().get()> <p class="text-gray-400">"Renaming..."</p> </Show>
                    <Show when=move || delete_item_action.pending().get()> <p class="text-gray-400">"Deleting..."</p> </Show>
                    // Show errors from actions
                    {move || view!{
                        {create_folder_action.value().get().map(|res| res.err().map(|e| view!{ <p class="text-red-400">"Create Error: "{e.to_string()}</p> }))}
                        {rename_item_action.value().get().map(|res| res.err().map(|e| view!{ <p class="text-red-400">"Rename Error: "{e.to_string()}</p> }))}
                        {delete_item_action.value().get().map(|res| res.err().map(|e| view!{ <p class="text-red-400">"Delete Error: "{e.to_string()}</p> }))}
                    }}
                </div>
            </div>
        </div>
    }
}

#[server]
pub async fn create_folder(project_slug: ProjectSlugStr, path: String, folder_name: String) -> Result<(), ServerFnError> {
    // TODO: Implement permission check and backend call (DirAction::Mkdir)
    log!("Server: Create folder '{}' in path '{}' for project '{}'", folder_name, path, project_slug);
    // 1. Get project_id from slug
    // 2. Check Permission::Write
    // 3. Construct full path: format!("{}/{}", path.trim_end_matches('/'), folder_name)
    // 4. Call request_server_project_action(slug, DirAction::Mkdir { path: full_path }.into())
    // 5. Handle response/errors
    Ok(())
}

#[server]
pub async fn rename_item(project_slug: ProjectSlugStr, path: String, old_name: String, new_name: String) -> Result<(), ServerFnError> {
    // TODO: Implement permission check and backend call (DirAction::Rename)
    log!("Server: Rename '{}' to '{}' in path '{}' for project '{}'", old_name, new_name, path, project_slug);
    // 1. Get project_id from slug
    // 2. Check Permission::Write
    // 3. Construct old_path and new_path
    // 4. Call request_server_project_action(slug, DirAction::Rename { from: old_path, to: new_path }.into())
    // 5. Handle response/errors
    Ok(())
}

#[server]
pub async fn delete_item(project_slug: ProjectSlugStr, path: String, name: String, is_dir: BoolInput) -> Result<(), ServerFnError> {
    // TODO: Implement permission check and backend call (DirAction::Rm)
    let is_dir = is_dir.0;
    log!("Server: Delete '{}' ({}) in path '{}' for project '{}'", name, if is_dir {"dir"} else {"file"}, path, project_slug);
    // 1. Get project_id from slug
    // 2. Check Permission::Write
    // 3. Construct item_path
    // 4. Call request_server_project_action(slug, DirAction::Rm { path: item_path, recursive: is_dir }.into()) // Assuming recursive delete for dirs
    // 5. Handle response/errors
    Ok(())
}


#[server]
pub async fn get_list_file(
    project_slug: ProjectSlugStr,
    path: String,
) -> Result<Vec<LsElement>, ServerFnError> {
    use crate::api::ssr::request_server_project_action;
    let project_id = ProjectSlug::from_str(project_slug.as_str()).map_err(|e| {
        leptos_axum::redirect("/user/projects");
        ServerFnError::new(e.to_string())
    })?.id;
    let auth = crate::ssr::auth(false)?;
    
    match crate::security::permission::ssr::ensure_permission(&auth, project_id, Permission::Read).await? {
        PermissionResult::Allow => {
            let pool = crate::ssr::pool()?;
            let project = sqlx::query!(
                r#"SELECT id, name FROM projects WHERE id = $1"#,
                project_id
            ).fetch_one(&pool).await.map_err(|e| ServerFnError::new(e.to_string()))?;
            let project_slug = ProjectSlug::new(project.id, project.name);
            match request_server_project_action(project_slug, DirAction::Ls {
                path
            }.into()).await.map_err(|e| ServerFnError::new(e.to_string()))?{
                ServerProjectActionResponse::Ls(inner) => Ok(inner.inner),
                _ => {
                    Err(ServerFnError::new("Invalid response"))
                }
            }
            
        }
        PermissionResult::Redirect(path) => {
            leptos_axum::redirect(path.as_str());
            Err(ServerFnError::new("Permission denied"))
        }
    }
    
}


#[server]
pub async fn get_file_content(
    project_slug: ProjectSlugStr,
    file_path: String, // Expects internal format like "./path/to/file.txt"
) -> Result<FileInfo, ServerFnError> {
    use crate::api::ssr::request_server_project_action; // Ensure import

    log!("Server: Get content for file '{}' in project '{}'", file_path, project_slug);

    // --- Path Handling for Backend ---
    // Adjust this based on what your backend `ReadFile` action expects
    let backend_path = if file_path.starts_with("./") {
        file_path.trim_start_matches("./").to_string()
    } else if file_path == "." {
        // This function shouldn't be called with "." but handle defensively
        return Err(ServerFnError::new("Cannot get content of root directory marker."))
    } else {
        // Path doesn't start with "./", might be an issue or different convention
        log!("Warning: get_file_content received path without expected './' prefix: {}", file_path);
        file_path.clone() // Pass as is, or handle error
    };
    // --- End Path Handling ---

    let project_id = ProjectSlug::from_str(project_slug.as_str())
        .map_err(|e| ServerFnError::new(format!("Invalid project slug: {}", e)))?
        .id;

    let auth = crate::ssr::auth(false)?;

    match crate::security::permission::ssr::ensure_permission(&auth, project_id, Permission::Read).await? {
        PermissionResult::Allow => {
            let pool = crate::ssr::pool()?;
            let project = sqlx::query!(
                r#"SELECT id, name FROM projects WHERE id = $1"#,
                project_id
            ).fetch_one(&pool).await.map_err(|e| ServerFnError::new(e.to_string()))?;

            let project_slug_obj = ProjectSlug::new(project.id, project.name);

            // --- Backend Call ---
            // TODO: Replace with actual backend call using DirAction::ReadFile or similar
            // let response = request_server_project_action(project_slug_obj, DirAction::ReadFile { path: backend_path }.into()).await?;
            // match response {
            //     ServerProjectActionResponse::FileContent(content_data) => {
            //         Ok(FileInfo {
            //              name: backend_path.split('/').last().unwrap_or_default().to_string(), // Basic name extraction
            //              path: file_path, // Return the originally requested path
            //              content: content_data.content, // Assuming content_data has a content field
            //         })
            //      },
            //      _ => Err(ServerFnError::new("Invalid response type when reading file"))
            // }
            // --- Placeholder Implementation ---
            log!("TODO: Implement actual backend call for DirAction::ReadFile");
            Ok(FileInfo {
                name: backend_path.split('/').last().unwrap_or_default().to_string(),
                path: file_path.clone(),
                content: format!("Placeholder content for file: {}\nBackend path: {}", file_path, backend_path),
            })
            // --- End Placeholder ---
        }
        PermissionResult::Redirect(_) => {
            Err(ServerFnError::new("Permission denied"))
        }
    }
}

