use leptos::{component, view, IntoView};
use leptos::prelude::{ActionForm, ClassAttribute, ServerAction};
use crate::app::components::csrf_field::CSRFField;
use leptos::prelude::ElementChild;
use crate::security::signup::Signup;

#[component]
pub  fn SignupPage(action: ServerAction<Signup>) -> impl IntoView {
    view! {
        <div class="flex min-h-full flex-col justify-center px-6 py-12 lg:px-8">
            <div class="sm:mx-auto sm:w-full sm:max-w-sm">
                <img
                    class="mx-auto h-10 w-auto"
                    src="https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=600"
                    alt="Hive Host"
                />
                <h2 class="mt-10 text-center text-2xl/9 font-bold tracking-tight text-gray-900">
                    Create an account
                </h2>
            </div>

            <div class="mt-10 sm:mx-auto sm:w-full sm:max-w-sm">
                <ActionForm action=action>
                    <CSRFField />
                    <div>
                        <label class="block text-sm/6 font-medium text-gray-900">
                            Email address <div class="mt-2">
                                <input
                                    type="email"
                                    name="email"
                                    autocomplete="email"
                                    required
                                    class="block w-full rounded-md bg-white px-3 py-1.5 text-base text-gray-900 outline-1 -outline-offset-1 outline-gray-300 placeholder:text-gray-400 focus:outline-2 focus:-outline-offset-2 focus:outline-indigo-600 sm:text-sm/6"
                                />
                            </div>
                        </label>
                    </div>
                    <div>
                        <label class="block text-sm/6 font-medium text-gray-900">
                            Username <div class="mt-2">
                                <input
                                    type="text"
                                    name="username"
                                    autocomplete="username"
                                    required
                                    class="block w-full rounded-md bg-white px-3 py-1.5 text-base text-gray-900 outline-1 -outline-offset-1 outline-gray-300 placeholder:text-gray-400 focus:outline-2 focus:-outline-offset-2 focus:outline-indigo-600 sm:text-sm/6"
                                />
                            </div>
                        </label>
                    </div>

                    <div>
                        <label class="block text-sm/6 font-medium text-gray-900">
                            Password <div class="mt-2">
                                <input
                                    type="password"
                                    name="password"
                                    required
                                    class="block w-full rounded-md bg-white px-3 py-1.5 text-base text-gray-900 outline-1 -outline-offset-1 outline-gray-300 placeholder:text-gray-400 focus:outline-2 focus:-outline-offset-2 focus:outline-indigo-600 sm:text-sm/6"
                                />
                            </div>
                        </label>

                    </div>
                    <div>
                        <label class="block text-sm/6 font-medium text-gray-900">
                            Confirm Password <div class="mt-2">
                                <input
                                    type="password"
                                    name="password_confirmation"
                                    required
                                    class="block w-full rounded-md bg-white px-3 py-1.5 text-base text-gray-900 outline-1 -outline-offset-1 outline-gray-300 placeholder:text-gray-400 focus:outline-2 focus:-outline-offset-2 focus:outline-indigo-600 sm:text-sm/6"
                                />
                            </div>
                        </label>

                    </div>

                    <div>
                        <label class="block text-sm/6 font-medium text-gray-900 flex">
                            Remember me <div class="flex h-6 shrink-0 items-center">
                                <div class="ml-2 group grid size-4 grid-cols-1">
                                    <input
                                        name="remember"
                                        type="checkbox"
                                        checked
                                        class="col-start-1 row-start-1 appearance-none rounded-sm border border-gray-300 bg-white checked:border-indigo-600 checked:bg-indigo-600 indeterminate:border-indigo-600 indeterminate:bg-indigo-600 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600 disabled:border-gray-300 disabled:bg-gray-100 disabled:checked:bg-gray-100 forced-colors:appearance-auto"
                                    />
                                    <svg
                                        class="pointer-events-none col-start-1 row-start-1 size-3.5 self-center justify-self-center stroke-white group-has-disabled:stroke-gray-950/25"
                                        viewBox="0 0 14 14"
                                        fill="none"
                                    >
                                        <path
                                            class="opacity-0 group-has-checked:opacity-100"
                                            d="M3 8L6 11L11 3.5"
                                            stroke-width="2"
                                            stroke-linecap="round"
                                            stroke-linejoin="round"
                                        />
                                        <path
                                            class="opacity-0 group-has-indeterminate:opacity-100"
                                            d="M3 7H11"
                                            stroke-width="2"
                                            stroke-linecap="round"
                                            stroke-linejoin="round"
                                        />
                                    </svg>
                                </div>
                            </div>
                        </label>
                    </div>
                    <div>
                        <button
                            type="submit"
                            class="flex w-full justify-center rounded-md bg-indigo-600 px-3 py-1.5 text-sm/6 font-semibold text-white shadow-xs hover:bg-indigo-500 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"
                        >
                            Sign up
                        </button>
                    </div>
                </ActionForm>
            </div>
        </div>
    }
}

