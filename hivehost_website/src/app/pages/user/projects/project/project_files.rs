use leptos::prelude::CustomAttribute;
use leptos::prelude::{AddAnyAttr, For, NodeRef, NodeRefAttribute, OnAttribute, RwSignal, Set, Show};
use leptos::prelude::{Effect, IntoAnyAttribute};
pub mod file_content_view;
pub mod project_files_sidebar;

use crate::api::{get_action_server_project_action, get_action_token_action};
use crate::app::pages::user::projects::project::project_files::file_content_view::FileContentView;
use crate::app::pages::user::projects::project::project_files::project_files_sidebar::ProjectFilesSidebar;
use crate::app::pages::user::projects::project::ProjectSlugSignal;
use crate::app::IntoView;
use leptos_router::params::Params;

use common::server_action::project_action::io_action::dir_action::ProjectIoDirAction;
use common::server_action::project_action::ProjectResponse;
use leptos::either::Either;

use leptos::prelude::{ElementChild, Memo, Read, Suspend, Transition};

use crate::app::pages::{GlobalState, GlobalStateStoreFields, ProjectStateStoreFields};
use crate::security::permission::request_server_project_action_front;
use common::server_action::project_action::io_action::file_action::ProjectIoFileAction;
use common::server_action::token_action::{TokenAction, UsedTokenActionResponse};
use common::{ProjectSlugStr, ServerId};
use leptos::html::Input;
use leptos::logging::log;
use leptos::prelude::{expect_context, signal, ClassAttribute, CollectView, Get, IntoMaybeErased};
use leptos::prelude::{Callback, Signal};
use leptos::reactive::spawn_local;
use leptos::server::Resource;
use leptos::{component, view, Params};
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_params};
use leptos_router::params::ParamsError;
use reactive_stores::{OptionStoreExt, Store};
use wasm_bindgen::JsCast;
use web_sys::{FormData, HtmlFormElement, SubmitEvent};

#[derive(Params, Clone, Debug, PartialEq)]
pub struct ProjectFilesParams {
    pub path: String,
}

pub type MemoProjectFilesParams = Memo<Result<ProjectFilesParams, ParamsError>>;

#[component]
pub fn ProjectFiles() -> impl IntoView {
    let params: MemoProjectFilesParams = use_params::<ProjectFilesParams>();
    let current_path = Signal::derive(move || {
        params
            .read()
            .as_ref()
            .ok()
            .map(|p| {
                let p = p.path.clone();
                if p.is_empty() {
                    return "root/".to_string();
                }
                let end_with_slash = p.ends_with("/");
                if end_with_slash {
                    p
                } else {
                    let mut p = p.clone();
                    p.push('/');
                    p
                }
            })
            .unwrap_or_else(|| "root/".to_string())
    });


    let global_state: Store<GlobalState> = expect_context();

    let project_slug_signal: Signal<ProjectSlugSignal> = expect_context();
    let permission_signal = Signal::derive(move || {
        global_state
            .project_state()
            .unwrap()
            .read()
            .permission
    });

    log!("Perm slug signal: {:?}", permission_signal.get());

    let slug = Signal::derive(move || project_slug_signal.read().0.clone());
    let server_id = Signal::derive(move || global_state.project_state().unwrap().project().read().server_id);


    let csrf_signal = Signal::derive(move || global_state.csrf().get());

    let (selected_file, set_selected_file) = signal::<Option<String>>(None);
    let refresh_signal = RwSignal::new(0u32);

    let server_project_action = get_action_server_project_action();
    let file_list_resource = Resource::new_bincode(
        move || {
            (
                refresh_signal.get(),
                current_path.get(),
                server_id(),
                slug(),
                server_project_action.version().get(),
            )
        },
        |(_,path,server_id, slug, _)| {
            request_server_project_action_front(
                server_id,
                slug,
                ProjectIoDirAction::Ls { path }.into(),
                None,
            )
        },
    );

    let breadcrumbs = move || {
        let path = current_path.get();
        let mut segments = vec![("Root".to_string(), "root/".to_string())];
        if path != "root/" {
            let mut accumulated_path = String::from("root");
            let relative_path = path.trim_start_matches("root/");
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
    Effect::new(move |_| {
        let _ = current_path.get();
        set_selected_file(None);
    });

    let handle_select_file = Callback::new(move |file_path: String| {
        set_selected_file(Some(file_path));
    });
    let folder_name_ref: NodeRef<Input> = NodeRef::new();
    let file_name_ref: NodeRef<Input> = NodeRef::new();

    let on_folder_create_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        let folder_name = folder_name_ref.get().unwrap().value();
        if folder_name.trim().is_empty() {
            return;
        }
        server_project_action.dispatch((
            server_id(),
            slug(),
            ProjectIoDirAction::Create {
                path: format!("{}{}", current_path.get(), folder_name),
            }
                .into(),
            Some(
                csrf_signal
                    .read()
                    .as_ref()
                    .map(|csrf| csrf.clone())
                    .unwrap_or_default(),
            ),
        ));
    };

    let on_file_create_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        let file_name = file_name_ref.get().unwrap().value();
        if file_name.trim().is_empty() {
            return;
        }
        server_project_action.dispatch((
                                           server_id(),
            slug(),
            ProjectIoFileAction::Create {
                path: format!("{}{}", current_path.get(), file_name),
            }
                .into(),
            Some(
                csrf_signal
                    .read()
                    .as_ref()
                    .map(|csrf| csrf.clone())
                    .unwrap_or_default(),
            ),
        ));
    };

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
                                                    <A
                                                        attr:class="hover:text-white hover:underline whitespace-nowrap"
                                                        href=move || {
                                                            format!("/user/projects/{}/files/{}", slug(), target_path)
                                                        }
                                                    >
                                                        {name}
                                                    </A>
                                                },
                                            )
                                        }}
                                    </span>
                                }
                            })
                            .collect_view()
                    }}
                </nav>
            </div>
            <div
                class="flex-shrink-0 p-4 border-b border-gray-700"
                class=("hidden", move || !permission_signal().can_edit())
            >
                <div class="grid grid-cols-1 md:grid-cols-3 gap-4 items-start">
                    <form on:submit=on_folder_create_submit class="flex flex-col space-y-2">
                        <label for="folder_name" class="form-label text-xs">
                            "New Folder"
                        </label>
                        <div class="flex items-center gap-x-2">
                            <input
                                type="text"
                                name="folder_name"
                                node_ref=folder_name_ref
                                class="form-input flex-grow"
                                placeholder="Folder name..."
                            />
                            <button type="submit" class="btn btn-primary flex-shrink-0">
                                <svg
                                    xmlns="http://www.w3.org/2000/svg"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    stroke-width="1.5"
                                    stroke="currentColor"
                                    class="w-5 h-5 mr-1"
                                >
                                    <path
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        d="M12 10.5v6m3-3H9m4.06-7.19-2.12-2.12a1.5 1.5 0 0 0-1.061-.44H4.5A2.25 2.25 0 0 0 2.25 6v12a2.25 2.25 0 0 0 2.25 2.25h15A2.25 2.25 0 0 0 21.75 18V9a2.25 2.25 0 0 0-2.25-2.25h-5.379a1.5 1.5 0 0 1-1.06-.44Z"
                                    />
                                </svg>
                                "Create"
                            </button>
                        </div>
                    </form>
                    <form on:submit=on_file_create_submit class="flex flex-col space-y-2">
                        <label for="file_name" class="form-label text-xs">
                            "New Empty File"
                        </label>
                        <div class="flex items-center gap-x-2">
                            <input
                                type="text"
                                name="file_name"
                                node_ref=file_name_ref
                                class="form-input flex-grow"
                                placeholder="File name..."
                            />
                            <button type="submit" class="btn btn-primary flex-shrink-0">
                                <svg
                                    xmlns="http://www.w3.org/2000/svg"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    stroke-width="1.5"
                                    stroke="currentColor"
                                    class="w-5 h-5 mr-1"
                                >
                                    <path
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m.75 12 3 3m0 0 3-3m-3 3v-6m-1.5-9H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z"
                                    />
                                </svg>
                                "Create"
                            </button>
                        </div>
                    </form>
        
                    <FileUploadArea slug current_path csrf_signal refresh_signal server_id />
                </div>
            </div>

            <div class="flex flex-grow overflow-hidden">

                <Transition fallback=move || {
                    view! { Loading... }
                }>
                    {move || {
                        Suspend::new(async move {
                            let (file_list, _) = signal(
                                file_list_resource
                                    .get()
                                    .and_then(|r| {
                                        r.ok()
                                            .and_then(|r| match r {
                                                ProjectResponse::Ls(inner) => Some(inner.inner),
                                                _ => None,
                                            })
                                    }),
                            );
                            Effect::new(move |_| {
                                if file_list.read().as_ref().is_none() {
                                    let navigate = use_navigate();
                                    navigate(
                                        &format!("/user/projects/{}/files/root/", slug()),
                                        Default::default(),
                                    );
                                }
                            });

                            view! {
                                <div class="w-64 md:w-80 flex-shrink-0 border-r border-white/10 overflow-y-auto">
                                    <ProjectFilesSidebar
                                        csrf_signal
                                        file_list=file_list
                                        current_path=current_path
                                        slug=slug
                                        on_select_file=handle_select_file
                                        server_project_action=server_project_action
                                        permission_signal=permission_signal
                                        server_id
                                    />

                                </div>
                            }
                        })
                    }}
                </Transition>
                <div class="flex-grow overflow-y-auto p-4 md:p-6 lg:p-8">
                    <FileContentView
                        csrf_signal
                        selected_file=selected_file.into()
                        slug=slug
                        permission_signal=permission_signal
                        server_id
                    />
                </div>
            </div>
        </div>
    }
}


#[component]
pub fn FileUploadArea(
    server_id: Signal<ServerId>,
    slug: Signal<ProjectSlugStr>,
    
    current_path: Signal<String>,
    csrf_signal: Signal<Option<String>>,
    refresh_signal:RwSignal<u32>
) -> impl IntoView {
    let file_input_ref: NodeRef<Input> = NodeRef::new();
    let (upload_messages, set_upload_messages) = signal(Vec::<String>::new());
    let (is_uploading, set_is_uploading) = signal(false);

    let handle_file_upload = move |ev: SubmitEvent| {
        ev.prevent_default();
        if let Some(input_element) = file_input_ref.get() {
            if let Some(file_list) = input_element.files() {
                if file_list.length() == 0 {
                    set_upload_messages(vec!["No files selected.".to_string()]);
                    return;
                }
                set_is_uploading(true);
                set_upload_messages(vec!["Starting upload...".to_string()]);
                let form_element = ev.target().unwrap().unchecked_into::<HtmlFormElement>();
                let form_data = FormData::new().unwrap();
                log!("File list length: {}", file_list.length());
                for i in 0..file_list.length() {
                    if let Some(file) = file_list.item(i) {
                        form_data.append_with_blob_and_filename("files[]", &file, &file.name()).unwrap();
                    }
                }
                form_element.reset();


                spawn_local(async move {
                    match get_action_token_action(
                        server_id(),
                        slug(),
                        TokenAction::UploadFiles { path: current_path() },
                        csrf_signal(),
                        Some(form_data),
                    ).await {
                        Ok(UsedTokenActionResponse::UploadReport(report)) => {
                            let messages: Vec<String> = report.into_iter()
                                .map(|status| format!("{}: {} ({})",
                                                      if status.success { "SUCCESS" } else { "FAIL" },
                                                      status.filename,
                                                      status.message
                                ))
                                .collect();
                            set_upload_messages(messages);
                        }
                        Ok(UsedTokenActionResponse::Error(e)) => {
                            set_upload_messages(vec![format!("Upload failed: {}", e)]);
                        }
                        Err(e) => {
                            set_upload_messages(vec![format!("Server error during upload: {:?}", e)]);
                        }
                        _ => {
                            set_upload_messages(vec!["Upload finished with an unexpected response.".to_string()]);
                        }
                    }
                    set_is_uploading(false);
                    refresh_signal.set(refresh_signal.get() + 1);
                });
            }
        }
    };

    view! {
        <div class="flex flex-col space-y-2">
            <form on:submit=handle_file_upload class="space-y-3">
                <div>
                    <label for="file_upload_input" class="form-label text-xs mb-1">
                        "Upload Files"
                    </label>
                    <input
                        node_ref=file_input_ref

                        name="input-file"
                        type="file"
                        multiple

                        class="form-input file:mr-3 file:py-1.5 file:px-3 file:rounded-md file:border-0 file:text-sm file:font-semibold file:bg-indigo-600 file:text-white hover:file:bg-indigo-500 cursor-pointer focus:outline-none"
                    />
                </div>
                <button
                    type="submit"
                    class="btn btn-primary w-full"
                    disabled=move || is_uploading.get()
                >
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke-width="1.5"
                        stroke="currentColor"
                        class="w-5 h-5 mr-2"
                    >
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5m-13.5-9L12 3m0 0 4.5 4.5M12 3v13.5"
                        />
                    </svg>
                    {move || if is_uploading.get() { "Uploading..." } else { "Upload Selected" }}
                </button>
            </form>

            <Show when=move || !upload_messages.get().is_empty()>
                <div class="mt-2 p-3 bg-gray-800 rounded-md max-h-32 overflow-y-auto text-xs space-y-1 shadow">
                    <For
                        each=move || upload_messages.get()
                        key=|msg| msg.clone()
                        children=move |msg| {
                            let is_success = msg.starts_with("SUCCESS");
                            view! {
                                <p class:text-green-400=is_success class:text-red-400=!is_success>
                                    {msg}
                                </p>
                            }
                        }
                    />
                </div>
            </Show>
        </div>
    }
}

pub mod server_fns {
    cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
    }}
}
