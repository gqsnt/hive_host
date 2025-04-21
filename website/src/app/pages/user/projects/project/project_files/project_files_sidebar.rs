use leptos::prelude::{signal, AddAnyAttr, FromFormData, NodeRef, NodeRefAttribute, ReadSignal, RwSignal};
use leptos::prelude::IntoAnyAttribute;
use leptos::prelude::ElementChild;
use leptos::prelude::CustomAttribute;
use leptos::prelude::{Callable, Get, IntoMaybeErased, Transition};
use leptos::{component, view, IntoView};
use leptos::prelude::{ActionForm, ClassAttribute, CollectView, GlobalAttributes, OnAttribute, Resource, ServerAction, ServerFnError, Signal};
use common::ProjectSlugStr;
use common::server_project_action::io_action::dir_action::{DirAction, LsElement};
use leptos::callback::Callback;
use leptos::either::{Either, EitherOf3};
use leptos::html::Input;
use web_sys::SubmitEvent;
use common::server_project_action::io_action::file_action::FileAction;
use common::server_project_action::io_action::IoAction::Dir;
use crate::api::{get_action_server_project_action, ServerProjectActionFront};

#[component]
pub fn ProjectFilesSidebar(
    file_list_resource:Resource<Result<Vec<LsElement>,ServerFnError>>,
    current_path: Signal<String>,
    slug: Signal<ProjectSlugStr>,
    on_go_up: Callback<()>,
    on_navigate_dir: Callback<String>,
    on_select_file: Callback<String>,
    server_project_action:ServerProjectActionFront,
) -> impl IntoView {
    
    let folder_name_ref:NodeRef<Input>=  NodeRef::new();
    let file_name_ref:NodeRef<Input>=  NodeRef::new();
    
    let ls_server_project_action = get_action_server_project_action();
    
    let on_folder_create_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        let folder_name = folder_name_ref.get().unwrap().value();
        if folder_name.trim().is_empty() {
            return;
        }
        server_project_action.dispatch((slug(), DirAction::Create {
            path:format!("{}/{}", current_path.get(), folder_name),
        }.into(), None));
    };
    
    let on_file_create_submit = move |ev:SubmitEvent|{
        ev.prevent_default();
        let file_name = file_name_ref.get().unwrap().value();
        if file_name.trim().is_empty() {
            return;
        }
        server_project_action.dispatch((slug(), FileAction::Create {
            path:format!("{}/{}", current_path.get(), file_name),
        }.into(),None));
    };
    
    
    
    view! {
        <div class="p-4 h-full flex flex-col">

            // Create Folder Section
            <div class="mb-2 flex-shrink-0">
                <form on:submit=on_folder_create_submit  class="flex items-center gap-x-2">
                    <input
                        type="text"
                        name="folder_name"
                        node_ref=folder_name_ref
                        class="form-input flex-grow px-2 py-1 text-sm"
                        placeholder="New folder name..."
                    />
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
                </form>
            </div>

            <hr class="border-white/10 my-3 flex-shrink-0" />

            <div class="mb-4 flex-shrink-0">
                <form on:submit=on_file_create_submit class="space-y-2">
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
                            node_ref=file_name_ref
                            class="form-input flex-grow px-2 py-1 text-sm"
                            placeholder="New file name..."
                        />
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
                </form>
            </div>

            <hr class="border-white/10 my-3 flex-shrink-0" />
            <ProjectFilesSidebarList
                slug=slug
                current_path=current_path
                file_list_resource=file_list_resource
                on_go_up=on_go_up
                on_navigate_dir=on_navigate_dir
                on_select_file=on_select_file
                server_project_action=server_project_action
            />

        </div>
    }
}


#[component]
pub fn ProjectFilesSidebarList(
    slug:Signal<ProjectSlugStr>,
    current_path:Signal<String>,
    file_list_resource: Resource<Result<Vec<LsElement>,ServerFnError>>,
    on_go_up: Callback<()>,
    on_navigate_dir: Callback<String>,
    on_select_file: Callback<String>,
    server_project_action:ServerProjectActionFront,
    
)  -> impl IntoView{
    view! {
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
                        file_list_resource
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
                                                        view! {
                                                            <ProjectFilesSidebarItem
                                                                slug=slug
                                                                current_path=current_path
                                                                item=item
                                                                server_project_action=server_project_action
                                                                on_navigate_dir=on_navigate_dir
                                                                on_select_file=on_select_file
                                                            />
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
    }
}

#[component]
pub fn ProjectFilesSidebarItem(
    slug:Signal<ProjectSlugStr>,
    current_path:Signal<String>,
    item:LsElement,
    server_project_action:ServerProjectActionFront,
    on_navigate_dir: Callback<String>,
    on_select_file: Callback<String>,
    
)  -> impl IntoView{
    let (is_renaming_item, set_is_renaming_item) = signal(
        false,
    );
    let new_name_ref:NodeRef<Input>= NodeRef::new();
    let (item_name, _) = signal(item.name.clone());
    
    let on_delete_item_submit = move |ev:SubmitEvent|{
        ev.prevent_default();
        let path = format!("{}/{}", current_path.get(), item_name());
        let action = if item.is_dir{
            DirAction::Delete{
                path,
            }.into()
        }else{
            FileAction::Delete{
                path,
            }.into()
        };
        server_project_action.dispatch((slug(),action,None));

    };
    
    let on_rename_item_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        let new_name = new_name_ref.get().unwrap().value();
        let old_name = item_name();
        if new_name.trim().is_empty()
            || new_name == old_name
        {
            return;
        }
        let action = if item.is_dir{
            DirAction::Rename {
                path: format!("{}/{}", current_path.get(), old_name),
                new_name,
            }.into()
        }else{
            FileAction::Rename {
                path: format!("{}/{}", current_path.get(), old_name),
                new_name,
            }.into()
        };
        server_project_action.dispatch((slug(),action,None));
        set_is_renaming_item(false)
    };
    
    view! {
        <li class="group flex items-center justify-between gap-x-1 px-2 py-1.5 text-sm rounded-md text-gray-300 hover:bg-gray-700 hover:text-white">
            <form
                on:submit=on_rename_item_submit
                class=move || {
                    format!(
                        "flex items-center gap-x-2 flex-grow {}",
                        if !is_renaming_item.get() { "hidden" } else { "" },
                    )
                }
            >
                <input
                    type="text"
                    name="new_name"
                    node_ref=new_name_ref
                    value=item_name()
                    class="form-input flex-grow px-2 py-1 text-sm"
                />
                <button type="submit" class="p-1 text-green-400 hover:text-green-300" title="Save">
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
            </form>
            <button
                class=move || {
                    format!(
                        "flex items-center gap-x-2 overflow-hidden flex-grow text-left hover:text-white {}",
                        if is_renaming_item.get() { "hidden" } else { "" },
                    )
                }
                on:click=move |_| {
                    if item.is_dir {
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
                    {if item.is_dir {
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
                <form on:submit=on_delete_item_submit>
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
                </form>

            </div>
        </li>
    }
}