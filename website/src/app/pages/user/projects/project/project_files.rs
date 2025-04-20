use leptos::prelude::{request_animation_frame, Callable, Callback, FromFormData, IntoAnyAttribute, MaybeSignal, NodeRef, NodeRefAttribute, ReadSignal, Signal, Transition};
use leptos::prelude::AddAnyAttr;
use leptos::prelude::{event_target_value, ActionForm, CustomAttribute, GlobalAttributes, PropAttribute, Show};
use std::str::FromStr;
use crate::app::ServerFnError;
use common::server_project_action::io_action::dir_action::{DirAction, LsElement};
use leptos::prelude::{expect_context, signal, ClassAttribute, CollectView, Effect, For, Get, IntoMaybeErased, OnAttribute, RwSignal, ServerAction, Set, Suspend, Suspense};
use leptos::prelude::ElementChild;
use leptos::{component, server, view};
use leptos::either::{Either, EitherOf3, EitherOf4};
use leptos::form::FromFormDataError;
use leptos::html::Input;
use leptos::logging::log;
use leptos::server::Resource;
use leptos::svg::set;
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
    let slug  = Signal::derive(move || params.get().unwrap().project_slug.clone());

    let (current_path, set_current_path) = signal(".".to_string());
    let (selected_file, set_selected_file) = signal::<Option<String>>(None); 

    // --- Actions ---
    let create_folder_action = ServerAction::<CreateFolder>::new();
    let rename_item_action = ServerAction::<RenameItem>::new(); 
    let delete_item_action = ServerAction::<DeleteItem>::new();
    let create_file_action = ServerAction::<CreateFile>::new();


    let ls_files_resource = Resource::new(
        move || (
            current_path.get(),
            slug(), 
            create_folder_action.version().get(),
            rename_item_action.version().get(), 
            delete_item_action.version().get(),
        ),
        |(path, slug, _, _, _)|   {
            get_list_file(slug, path)
        }
    );


    let go_up_one_level = move |_| {
        let current = current_path.get();
        if current != "." {
            if let Some(last_slash_idx) = current.rfind('/') {
                if last_slash_idx == 1 {
                    set_current_path(".".to_string());
                } else {
                    set_current_path(current[..last_slash_idx].to_string());
                }
            }
        }
    };

   
    let breadcrumbs = move || {
        let path = current_path.get();
        let mut segments = vec![("Root".to_string(), ".".to_string())];
        if path != "." {
            let mut accumulated_path = String::from(".");
            let relative_path = path.trim_start_matches("./");
            for segment in relative_path.split('/') {
                if !segment.is_empty() {
                    accumulated_path.push('/');
                    accumulated_path.push_str(segment);
                    segments.push((segment.to_string(), accumulated_path.clone()));
                }
            }
        }
        segments
    };

   
    let handle_select_file = Callback::new(move |file_path: String| {
        log!("Selected file: {}", file_path);
        set_selected_file(Some(file_path));
    });


    let handle_navigate_dir = Callback::new(move |dir_path: String| {
        log!("Navigating to dir: {}", dir_path);
        set_current_path(dir_path);
        set_selected_file(None); 
    });


    view! {
        <div class="flex flex-col h-full">

            <div class="flex-shrink-0 p-4 border-b border-white/10">
                <nav class="flex items-center space-x-1 text-sm text-gray-400 flex-wrap">
                    {move || {
                        breadcrumbs()
                            .into_iter()
                            .enumerate()
                            .map(|(i, (name, target_path))| {
                                let is_last = i == breadcrumbs().len() - 1;
                                view! {
                                    <span class="flex items-center">
                                        {(i > 0).then(|| view! { <span class="mx-1">/</span> })}
                                        {if is_last {
                                            Either::Left(
                                                view! {
                                                    <span class="font-medium text-white whitespace-nowrap">
                                                        {name}
                                                    </span>
                                                },
                                            )
                                        } else {
                                            Either::Right(
                                                view! {
                                                    <button
                                                        class="hover:text-white hover:underline whitespace-nowrap"
                                                        on:click=move |_| set_current_path(target_path.clone())
                                                    >
                                                        {name}
                                                    </button>
                                                },
                                            )
                                        }}
                                    </span>
                                }
                            })
                            .collect_view()
                    }}
                </nav>
                <div class="mt-2 text-sm">
                    {move || {
                        view! {
                            {create_folder_action
                                .value()
                                .get()
                                .map(|res| {
                                    res.err()
                                        .map(|e| {
                                            view! {
                                                <p class="text-red-400">"Create Error: "{e.to_string()}</p>
                                            }
                                        })
                                })}
                            {rename_item_action
                                .value()
                                .get()
                                .map(|res| {
                                    res.err()
                                        .map(|e| {
                                            view! {
                                                <p class="text-red-400">"Rename Error: "{e.to_string()}</p>
                                            }
                                        })
                                })}
                            {delete_item_action
                                .value()
                                .get()
                                .map(|res| {
                                    res.err()
                                        .map(|e| {
                                            view! {
                                                <p class="text-red-400">"Delete Error: "{e.to_string()}</p>
                                            }
                                        })
                                })}
                        }
                    }}
                </div>
            </div>

            <div class="flex flex-grow overflow-hidden">
                <div class="w-64 md:w-80 flex-shrink-0 border-r border-white/10 overflow-y-auto">
                    <FileSidebar
                        files_list=ls_files_resource
                        current_path=current_path.into()
                        slug=slug
                        on_go_up=Callback::new(move |_| go_up_one_level(()))
                        on_navigate_dir=handle_navigate_dir
                        on_select_file=handle_select_file
                        create_folder_action=create_folder_action
                        delete_item_action=delete_item_action
                        create_file_action=create_file_action
                        rename_item_action=rename_item_action
                    />
                </div>
                <div class="flex-grow overflow-y-auto p-4 md:p-6 lg:p-8">
                    <FileContentView selected_file=selected_file.into() slug=slug />
                </div>
            </div>
        </div>
    }
}


#[component]
fn FileSidebar(
    files_list: Resource<Result<Vec<LsElement>,ServerFnError>>,
    current_path: Signal<String>,
    slug: Signal<ProjectSlugStr>,
    on_go_up: Callback<()>,
    on_navigate_dir: Callback<String>,
    on_select_file: Callback<String>,
    create_folder_action: ServerAction<CreateFolder>,
    delete_item_action: ServerAction<DeleteItem>,
    create_file_action: ServerAction<CreateFile>,
    rename_item_action: ServerAction<RenameItem>,
) -> impl IntoView {

    
   



    view! {
        <div class="p-4 h-full flex flex-col">

            // Create Folder Section
            <div class="mb-2 flex-shrink-0">
                <ActionForm action=create_folder_action attr:class="flex items-center gap-x-2">
                    <input
                        type="text"
                        name="folder_name"
                        class="form-input flex-grow px-2 py-1 text-sm"
                        placeholder="New folder name..."
                    />
                    <input type="hidden" name="project_slug" value=slug() />
                    <input type="hidden" name="path" value=current_path.get() />
                    <button type="submit" class="btn-primary text-sm px-2 py-1 flex-shrink-0">
                        <svg
                            xmlns="http://www.w3.org/2000/svg"
                            fill="none"
                            viewBox="0 0 24 24"
                            stroke-width="1.5"
                            stroke="currentColor"
                            class="w-4 h-4 inline mr-1"
                        >
                            <path
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                d="M12 10.5v6m3-3H9m4.06-7.19-2.12-2.12a1.5 1.5 0 0 0-1.061-.44H4.5A2.25 2.25 0 0 0 2.25 6v12a2.25 2.25 0 0 0 2.25 2.25h15A2.25 2.25 0 0 0 21.75 18V9a2.25 2.25 0 0 0-2.25-2.25h-5.379a1.5 1.5 0 0 1-1.06-.44Z"
                            />
                        </svg>
                        "Create"
                    </button>
                </ActionForm>
            </div>

            <hr class="border-white/10 my-3 flex-shrink-0" />

            <div class="mb-4 flex-shrink-0">
                <ActionForm action=create_file_action attr:class="space-y-2">
                    // File Input (for upload)
                    // <div>
                    // <label for="file_upload" class="form-label text-xs mb-1">Upload file (optional)</label>
                    // <input
                    // node_ref=file_input_ref
                    // id="file_upload"
                    // name="file_content" // Name for potential FormData
                    // type="file"
                    // class="block w-full text-sm text-gray-400 file:mr-4 file:py-1 file:px-2 file:rounded-md file:border-0 file:text-sm file:font-semibold file:bg-indigo-500 file:text-white hover:file:bg-indigo-400 cursor-pointer"
                    // />
                    // </div>
                    // Filename Input + Create Button Row
                    <div class="flex items-center gap-x-2">
                        <input
                            type="text"
                            name="file_name"
                            class="form-input flex-grow px-2 py-1 text-sm"
                            placeholder="New file name..."
                        />
                        <input type="hidden" name="project_slug" value=slug() />
                        <input type="hidden" name="path" value=current_path.get() />
                        <button type="submit" class="btn-primary text-sm px-2 py-1 flex-shrink-0">
                            <svg
                                xmlns="http://www.w3.org/2000/svg"
                                fill="none"
                                viewBox="0 0 24 24"
                                stroke-width="1.5"
                                stroke="currentColor"
                                class="w-4 h-4 inline mr-1"
                            >
                                <path
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                    d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m6.75 12.75h3m-3.75 0h.008v.008h-.008v-.008Zm0 0-.008.008h.008v-.008Zm-3.75 0h.008v.008h-.008v-.008Zm0 0-.008.008h.008v-.008ZM12 3.75 M12 3.75a.75.75 0 0 0-.75.75v10.5a.75.75 0 0 0 1.5 0V4.5A.75.75 0 0 0 12 3.75ZM3.75 12a.75.75 0 0 0 .75.75h10.5a.75.75 0 0 0 0-1.5H4.5a.75.75 0 0 0-.75.75Z"
                                />
                            </svg>
                            "Create / Upload"
                        </button>
                    </div>
                </ActionForm>
            </div>

            <hr class="border-white/10 my-3 flex-shrink-0" />

            <div class="flex-grow overflow-y-auto -mr-4 pr-4">
                <ul class="space-y-1">
                    {move || {
                        (current_path.get() != ".")
                            .then(|| {
                                view! {
                                    <li>
                                        <button
                                            class="flex items-center w-full gap-x-2 px-2 py-1.5 text-sm rounded-md text-indigo-400 hover:bg-gray-700 hover:text-indigo-300"
                                            on:click=move |_| {
                                                on_go_up.try_run(());
                                            }
                                        >
                                            <svg
                                                xmlns="http://www.w3.org/2000/svg"
                                                fill="none"
                                                viewBox="0 0 24 24"
                                                stroke-width="1.5"
                                                stroke="currentColor"
                                                class="w-5 h-5 flex-shrink-0"
                                            >
                                                <path
                                                    stroke-linecap="round"
                                                    stroke-linejoin="round"
                                                    d="M9 9l6-6m0 0l6 6m-6-6v12a6 6 0 01-12 0v-3"
                                                />
                                            </svg>
                                            <span>".."</span>
                                        </button>
                                    </li>
                                }
                            })
                    }}
                    <Transition fallback=move || {
                        view! { <li>Loading...</li> }
                    }>
                        {move || {
                            files_list
                                .get()
                                .map(|result| match result {
                                    Ok(ls_elements) => {
                                        Either::Left({
                                            if ls_elements.is_empty() && current_path.get() == "." {
                                                EitherOf3::A(view! {})
                                            } else if ls_elements.is_empty() {
                                                EitherOf3::B(
                                                    view! {
                                                        <li class="px-2 py-1.5 text-sm text-gray-500 italic">
                                                            "Folder is empty"
                                                        </li>
                                                    },
                                                )
                                            } else {
                                                EitherOf3::C(
                                                    ls_elements
                                                        .into_iter()
                                                        .map(|item| {
                                                            let (item_name, _) = signal(item.name);
                                                            let item_is_dir = item.is_dir;
                                                            let full_item_path = format!(
                                                                "{}/{}",
                                                                current_path.get().trim_end_matches('/'),
                                                                item_name(),
                                                            )
                                                                .replace("././", "./");
                                                            let (is_renaming_item, set_is_renaming_item) = signal(
                                                                false,
                                                            );
                                                            let submit_rename = move |ev: SubmitEvent| {
                                                                let event = RenameItem::from_event(&ev);
                                                                ev.prevent_default();
                                                                match event {
                                                                    Ok(event) => {
                                                                        if event.new_name.trim().is_empty()
                                                                            || event.new_name == event.old_name
                                                                        {
                                                                            return;
                                                                        } else {
                                                                            rename_item_action.dispatch(event);
                                                                        }
                                                                    }
                                                                    Err(_) => {
                                                                        return;
                                                                    }
                                                                };
                                                                set_is_renaming_item(false)
                                                            };

                                                            view! {
                                                                <li class="group flex items-center justify-between gap-x-1 px-2 py-1.5 text-sm rounded-md text-gray-300 hover:bg-gray-700 hover:text-white">
                                                                    <ActionForm
                                                                        action=rename_item_action
                                                                        attr:class=move || {
                                                                            format!(
                                                                                "flex items-center gap-x-2 flex-grow {}",
                                                                                if !is_renaming_item.get() { "hidden" } else { "" },
                                                                            )
                                                                        }
                                                                        on:submit=submit_rename
                                                                    >
                                                                        <input type="hidden" name="project_slug" value=slug() />
                                                                        // Dir path
                                                                        <input type="hidden" name="path" value=current_path.get() />
                                                                        // Pass cloned name
                                                                        <input type="hidden" name="old_name" value=item_name() />
                                                                        // Pass is_dir for server side if needed
                                                                        <input
                                                                            type="hidden"
                                                                            name="is_dir"
                                                                            value=item_is_dir.to_string()
                                                                        />
                                                                        <input
                                                                            type="text"
                                                                            name="new_name"
                                                                            value=item_name()
                                                                            class="form-input flex-grow px-2 py-1 text-sm"
                                                                        />
                                                                        <button
                                                                            type="submit"
                                                                            class="p-1 text-green-400 hover:text-green-300"
                                                                            title="Save"
                                                                        >
                                                                            <svg
                                                                                xmlns="http://www.w3.org/2000/svg"
                                                                                fill="none"
                                                                                viewBox="0 0 24 24"
                                                                                stroke-width="1.5"
                                                                                stroke="currentColor"
                                                                                class="w-5 h-5"
                                                                            >
                                                                                <path
                                                                                    stroke-linecap="round"
                                                                                    stroke-linejoin="round"
                                                                                    d="M4.5 12.75l6 6 9-13.5"
                                                                                />
                                                                            </svg>
                                                                        </button>
                                                                        <button
                                                                            type="button"
                                                                            on:click=move |e| {
                                                                                e.prevent_default();
                                                                                set_is_renaming_item(false)
                                                                            }
                                                                            class="p-1 text-gray-400 hover:text-white"
                                                                            title="Cancel"
                                                                        >
                                                                            <svg
                                                                                xmlns="http://www.w3.org/2000/svg"
                                                                                fill="none"
                                                                                viewBox="0 0 24 24"
                                                                                stroke-width="1.5"
                                                                                stroke="currentColor"
                                                                                class="w-5 h-5"
                                                                            >
                                                                                <path
                                                                                    stroke-linecap="round"
                                                                                    stroke-linejoin="round"
                                                                                    d="M6 18L18 6M6 6l12 12"
                                                                                />
                                                                            </svg>
                                                                        </button>
                                                                    </ActionForm>
                                                                    <button
                                                                        class=move || {
                                                                            format!(
                                                                                "flex items-center gap-x-2 overflow-hidden flex-grow text-left hover:text-white {}",
                                                                                if is_renaming_item.get() { "hidden" } else { "" },
                                                                            )
                                                                        }
                                                                        on:click=move |_| {
                                                                            if item_is_dir {
                                                                                let next_path = if current_path.get() == "." {
                                                                                    format!("./{}", item_name())
                                                                                } else {
                                                                                    format!("{}/{}", current_path.get(), item_name())
                                                                                };
                                                                                on_navigate_dir.try_run(next_path);
                                                                            } else {
                                                                                let full_item_path = format!(
                                                                                    "{}/{}",
                                                                                    current_path.get().trim_end_matches('/'),
                                                                                    item_name(),
                                                                                )
                                                                                    .replace("././", "./");
                                                                                on_select_file.try_run(full_item_path);
                                                                            }
                                                                        }
                                                                    >
                                                                        // Icon container
                                                                        <span class="flex-shrink-0 w-5 h-5">
                                                                            {if item_is_dir {
                                                                                Either::Left(
                                                                                    view! {
                                                                                        <svg
                                                                                            xmlns="http://www.w3.org/2000/svg"
                                                                                            fill="none"
                                                                                            viewBox="0 0 24 24"
                                                                                            stroke-width="1.5"
                                                                                            stroke="currentColor"
                                                                                            class="w-5 h-5 text-sky-400"
                                                                                        >
                                                                                            <path
                                                                                                stroke-linecap="round"
                                                                                                stroke-linejoin="round"
                                                                                                d="M2.25 12.75V12A2.25 2.25 0 014.5 9.75h15A2.25 2.25 0 0121.75 12v.75m-8.69-6.44l-2.12-2.12a1.5 1.5 0 00-1.061-.44H4.5A2.25 2.25 0 002.25 6v12a2.25 2.25 0 002.25 2.25h15A2.25 2.25 0 0021.75 18V9a2.25 2.25 0 00-2.25-2.25h-5.379a1.5 1.5 0 01-1.06-.44z"
                                                                                            />
                                                                                        </svg>
                                                                                    },
                                                                                )
                                                                            } else {
                                                                                Either::Right(
                                                                                    view! {
                                                                                        <svg
                                                                                            xmlns="http://www.w3.org/2000/svg"
                                                                                            fill="none"
                                                                                            viewBox="0 0 24 24"
                                                                                            stroke-width="1.5"
                                                                                            stroke="currentColor"
                                                                                            class="w-5 h-5 text-gray-400"
                                                                                        >
                                                                                            <path
                                                                                                stroke-linecap="round"
                                                                                                stroke-linejoin="round"
                                                                                                d="M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m2.25 0H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9z"
                                                                                            />
                                                                                        </svg>
                                                                                    },
                                                                                )
                                                                            }}
                                                                        </span>
                                                                        <span class="truncate flex-grow">{item_name()}</span>
                                                                    </button>

                                                                    <div class=move || {
                                                                        format!(
                                                                            "flex-shrink-0 opacity-0 group-hover:opacity-100 transition-opacity flex items-center gap-x-1 {}",
                                                                            if is_renaming_item.get() { "hidden" } else { "" },
                                                                        )
                                                                    }>
                                                                        <button
                                                                            type="button"
                                                                            class="p-0.5 text-gray-400 hover:text-white"
                                                                            title="Rename"
                                                                            on:click=move |ev| {
                                                                                ev.stop_propagation();
                                                                                set_is_renaming_item(true);
                                                                            }
                                                                        >
                                                                            <svg
                                                                                xmlns="http://www.w3.org/2000/svg"
                                                                                fill="none"
                                                                                viewBox="0 0 24 24"
                                                                                stroke-width="1.5"
                                                                                stroke="currentColor"
                                                                                class="w-4 h-4"
                                                                            >
                                                                                <path
                                                                                    stroke-linecap="round"
                                                                                    stroke-linejoin="round"
                                                                                    d="M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L6.832 19.82a4.5 4.5 0 01-1.897 1.13l-2.685.8.8-2.685a4.5 4.5 0 011.13-1.897L16.863 4.487zm0 0L19.5 7.125"
                                                                                />
                                                                            </svg>
                                                                        </button>
                                                                        <ActionForm action=delete_item_action>
                                                                            <input type="hidden" name="project_slug" value=slug() />
                                                                            <input type="hidden" name="path" value=current_path.get() />
                                                                            <input type="hidden" name="item_name" value=item_name() />
                                                                            <input
                                                                                type="hidden"
                                                                                name="is_dir"
                                                                                value=item_is_dir.to_string()
                                                                            />
                                                                            <button
                                                                                type="submit"
                                                                                class="p-0.5 text-red-500 hover:text-red-400"
                                                                                title="Delete"
                                                                            >
                                                                                <svg
                                                                                    xmlns="http://www.w3.org/2000/svg"
                                                                                    fill="none"
                                                                                    viewBox="0 0 24 24"
                                                                                    stroke-width="1.5"
                                                                                    stroke="currentColor"
                                                                                    class="w-4 h-4"
                                                                                >
                                                                                    <path
                                                                                        stroke-linecap="round"
                                                                                        stroke-linejoin="round"
                                                                                        d="M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0"
                                                                                    />
                                                                                </svg>
                                                                            </button>
                                                                        </ActionForm>

                                                                    </div>
                                                                </li>
                                                            }
                                                        })
                                                        .collect_view(),
                                                )
                                            }
                                        })
                                    }
                                    Err(e) => {
                                        Either::Right(
                                            // Collect into a view
                                            view! {
                                                <li class="px-2 py-1.5 text-sm text-red-400">
                                                    "Error: " {e.to_string()}
                                                </li>
                                            },
                                        )
                                    }
                                })
                        }}
                    </Transition>
                </ul>
            </div>
        </div>
    }
}

#[component]
fn FileContentView(
    selected_file: Signal<Option<String>>,
    slug: Signal<ProjectSlugStr>,
) -> impl IntoView {
    let file_content_resource = Resource::new(
        move || (selected_file.get(), slug.get()),
        |(file_path_opt, slug)| async move {
            match file_path_opt {
                Some(file_path) => get_file_content(slug, file_path).await,
                _ => Ok(None),
            }
        }
    );


    view! {
        <Transition fallback=move || {
            view! { <p class="text-gray-400">"Loading file content..."</p> }
        }>
            {move || {
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
                                        Ok(Some(file_info)) => {
                                            EitherOf3::A({
                                                let (file_info_name, _) = signal(file_info.name.clone());
                                                view! {
                                                    <div>
                                                        <div class="mb-4 pb-4 border-b border-white/10">
                                                            <h3 class="text-lg font-medium text-white truncate">
                                                                {file_info.name}
                                                            </h3>
                                                        </div>
                                                        <pre class="bg-gray-900 p-4 rounded-md text-sm text-gray-200 overflow-x-auto">
                                                            <code>{file_info.content}</code>
                                                        </pre>
                                                    </div>
                                                }
                                            })
                                        }
                                        Ok(None) => {
                                            EitherOf3::B(

                                                view! {
                                                    <p class="text-gray-400">"Waiting for file data..."</p>
                                                },
                                            )
                                        }
                                        Err(e) => {
                                            EitherOf3::C(
                                                view! {
                                                    <div class="text-red-400">
                                                        <p>"Error loading file content:"</p>
                                                        <p>{e.to_string()}</p>
                                                    </div>
                                                },
                                            )
                                        }
                                    }
                                }),
                        )
                    }
                }
            }}
        </Transition>
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
pub async fn create_file(project_slug: ProjectSlugStr, path: String,file_name:String) -> Result<(), ServerFnError> {
    // TODO: Implement permission check and backend call (DirAction::Mkdir)
    log!("Server: Create File '{}' in path '{}' for project '{}'", file_name, path, project_slug);
    // 1. Get project_id from slug
    // 2. Check Permission::Write
    // 3. Construct full path: format!("{}/{}", path.trim_end_matches('/'), folder_name)
    // 4. Call request_server_project_action(slug, DirAction::Mkdir { path: full_path }.into())
    // 5. Handle response/errors
    Ok(())
}


#[server]
pub async fn rename_item(project_slug: ProjectSlugStr, path: String, old_name: String, new_name: String, is_dir:BoolInput) -> Result<(), ServerFnError> {
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
pub async fn delete_item(project_slug: ProjectSlugStr, path: String, item_name: String, is_dir: BoolInput) -> Result<(), ServerFnError> {
    // TODO: Implement permission check and backend call (DirAction::Rm)
    let is_dir = is_dir.0;
    log!("Server: Delete '{}' ({}) in path '{}' for project '{}'", item_name, if is_dir {"dir"} else {"file"}, path, project_slug);
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
) -> Result<Option<FileInfo>, ServerFnError> {
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
            Ok(Some(FileInfo {
                name: backend_path.split('/').last().unwrap_or_default().to_string(),
                path: file_path.clone(),
                content: format!("Placeholder content for file: {}\nBackend path: {}", file_path, backend_path),
            }))
            // --- End Placeholder ---
        }
        PermissionResult::Redirect(_) => {
            Err(ServerFnError::new("Permission denied"))
        }
    }
}

