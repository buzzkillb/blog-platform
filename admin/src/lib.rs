use leptos::*;
use leptos_meta::*;
use leptos_router::*;

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Meta name="charset" content="UTF-8"/>
        <Meta name="viewport" content="width=device-width, initial-scale=1"/>
        <Stylesheet href="https://cdn.jsdelivr.net/npm/tailwindcss@2.2.19/dist/tailwind.min.css"/>

        <Router>
            <Routes>
                <Route path="/admin" view=AdminLayout>
                    <Route path="/sites" view=SitesPage/>
                    <Route path="/sites/:id" view=SiteDetailPage/>
                    <Route path="/sites/:id/posts" view=PostsPage/>
                    <Route path="/sites/:id/posts/new" view=PostEditorPage/>
                    <Route path="/sites/:id/posts/:post_id" view=PostEditorPage/>
                    <Route path="/sites/:id/pages" view=PagesPage/>
                    <Route path="/sites/:id/pages/new" view=PageEditorPage/>
                    <Route path="/sites/:id/pages/:page_id" view=PageEditorPage/>
                    <Route path="/sites/:id/media" view=MediaPage/>
                    <Route path="/sites/:id/contact" view=ContactSubmissionsPage/>
                </Route>
                <Route path="/admin/login" view=LoginPage/>
                <Route path="/" view=HomePage/>
            </Routes>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-gray-900 text-gray-100 flex items-center justify-center">
            <div class="text-center">
                <h1 class="text-4xl font-bold mb-4">Blog Platform</h1>
                <a href="/admin" class="text-blue-400 hover:underline">Go to Admin</a>
            </div>
        </div>
    }
}

#[component]
fn AdminLayout() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-gray-900 text-gray-100">
            <nav class="bg-gray-800 border-b border-gray-700 px-6 py-4">
                <div class="flex items-center justify-between">
                    <div class="flex items-center gap-6">
                        <a href="/admin/sites" class="text-xl font-bold text-white">
                            Blog Platform
                        </a>
                    </div>
                </div>
            </nav>
            <main class="p-6">
                <Outlet/>
            </main>
        </div>
    }
}

#[component]
fn LoginPage() -> impl IntoView {
    view! {
        <div class="min-h-screen flex items-center justify-center bg-gray-900">
            <div class="bg-gray-800 p-8 rounded-lg shadow-xl w-full max-w-md border border-gray-700">
                <h1 class="text-2xl font-bold text-white mb-6">Sign In</h1>
                <div>
                    <div class="mb-4">
                        <label class="block text-gray-400 mb-2">Email</label>
                        <input
                            type="email"
                            class="w-full bg-gray-700 border border-gray-600 rounded px-4 py-2 text-white focus:outline-none focus:border-blue-500"
                        />
                    </div>
                    <div class="mb-6">
                        <label class="block text-gray-400 mb-2">Password</label>
                        <input
                            type="password"
                            class="w-full bg-gray-700 border border-gray-600 rounded px-4 py-2 text-white focus:outline-none focus:border-blue-500"
                        />
                    </div>
                    <button class="w-full bg-blue-600 hover:bg-blue-700 text-white font-medium py-2 px-4 rounded transition">
                        Sign In
                    </button>
                </div>
            </div>
        </div>
    }
}

#[component]
fn SitesPage() -> impl IntoView {
    view! {
        <div>
            <div class="flex justify-between items-center mb-6">
                <h1 class="text-2xl font-bold text-white">Sites</h1>
                <button class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded">
                    New Site
                </button>
            </div>
            <div class="bg-gray-800 p-6 rounded-lg border border-gray-700 text-center">
                <p class="text-gray-400">No sites yet. Create your first site!</p>
            </div>
        </div>
    }
}

#[component]
fn SiteDetailPage() -> impl IntoView {
    view! {
        <div>
            <h1 class="text-2xl font-bold text-white mb-6">Site Dashboard</h1>
            <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
                <a href="#" class="bg-gray-800 hover:bg-gray-750 p-6 rounded-lg border border-gray-700 text-center">
                    <div class="text-3xl mb-2">Post</div>
                    <div class="text-white font-medium">Posts</div>
                </a>
                <a href="#" class="bg-gray-800 hover:bg-gray-750 p-6 rounded-lg border border-gray-700 text-center">
                    <div class="text-3xl mb-2">Page</div>
                    <div class="text-white font-medium">Pages</div>
                </a>
                <a href="#" class="bg-gray-800 hover:bg-gray-750 p-6 rounded-lg border border-gray-700 text-center">
                    <div class="text-3xl mb-2">Image</div>
                    <div class="text-white font-medium">Media</div>
                </a>
                <a href="#" class="bg-gray-800 hover:bg-gray-750 p-6 rounded-lg border border-gray-700 text-center">
                    <div class="text-3xl mb-2">Mail</div>
                    <div class="text-white font-medium">Contact</div>
                </a>
            </div>
        </div>
    }
}

#[component]
fn PostsPage() -> impl IntoView {
    view! {
        <div>
            <div class="flex justify-between items-center mb-6">
                <h1 class="text-2xl font-bold text-white">Posts</h1>
                <a href="#" class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded">
                    New Post
                </a>
            </div>
            <div class="bg-gray-800 p-6 rounded-lg border border-gray-700 text-center">
                <p class="text-gray-400">No posts yet. Create your first post!</p>
            </div>
        </div>
    }
}

#[component]
fn PostEditorPage() -> impl IntoView {
    view! {
        <div>
            <div class="flex justify-between items-center mb-6">
                <h1 class="text-2xl font-bold text-white">New Post</h1>
            </div>
            <div class="space-y-4">
                <div>
                    <label class="block text-gray-400 mb-2">Title</label>
                    <input
                        type="text"
                        class="w-full bg-gray-800 border border-gray-700 rounded px-4 py-3 text-white text-xl"
                        placeholder="Post title"
                    />
                </div>
                <div>
                    <label class="block text-gray-400 mb-2">Slug</label>
                    <input
                        type="text"
                        class="w-full bg-gray-800 border border-gray-700 rounded px-4 py-2 text-white"
                        placeholder="post-url-slug"
                    />
                </div>
                <div class="bg-gray-800 rounded-lg border border-gray-700 p-4">
                    <div class="flex gap-2 mb-4 flex-wrap">
                        <button class="px-3 py-1 bg-gray-700 hover:bg-gray-600 rounded text-sm">Text</button>
                        <button class="px-3 py-1 bg-gray-700 hover:bg-gray-600 rounded text-sm">Heading</button>
                        <button class="px-3 py-1 bg-gray-700 hover:bg-gray-600 rounded text-sm">Image</button>
                        <button class="px-3 py-1 bg-gray-700 hover:bg-gray-600 rounded text-sm">Code</button>
                    </div>
                    <textarea
                        class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white"
                        rows="8"
                        placeholder="Write your post content here..."
                    ></textarea>
                </div>
            </div>
        </div>
    }
}

#[component]
fn PagesPage() -> impl IntoView {
    view! {
        <div>
            <div class="flex justify-between items-center mb-6">
                <h1 class="text-2xl font-bold text-white">Pages</h1>
                <a href="#" class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded">
                    New Page
                </a>
            </div>
            <div class="bg-gray-800 p-6 rounded-lg border border-gray-700 text-center">
                <p class="text-gray-400">No pages yet.</p>
            </div>
        </div>
    }
}

#[component]
fn PageEditorPage() -> impl IntoView {
    view! {
        <div>
            <div class="flex justify-between items-center mb-6">
                <h1 class="text-2xl font-bold text-white">New Page</h1>
            </div>
            <div class="space-y-4">
                <div>
                    <label class="block text-gray-400 mb-2">Title</label>
                    <input
                        type="text"
                        class="w-full bg-gray-800 border border-gray-700 rounded px-4 py-3 text-white text-xl"
                        placeholder="Page title"
                    />
                </div>
                <div>
                    <label class="block text-gray-400 mb-2">Slug</label>
                    <input
                        type="text"
                        class="w-full bg-gray-800 border border-gray-700 rounded px-4 py-2 text-white"
                        placeholder="page-url-slug"
                    />
                </div>
            </div>
        </div>
    }
}

#[component]
fn MediaPage() -> impl IntoView {
    view! {
        <div>
            <h1 class="text-2xl font-bold text-white mb-6">Media Library</h1>
            <div class="bg-gray-800 rounded-lg border border-gray-700 p-6 mb-6">
                <label class="block">
                    <span class="text-gray-400 mb-2 block">Upload new file</span>
                    <input
                        type="file"
                        multiple
                        class="block w-full text-sm text-gray-400"
                    />
                </label>
            </div>
        </div>
    }
}

#[component]
fn ContactSubmissionsPage() -> impl IntoView {
    view! {
        <div>
            <h1 class="text-2xl font-bold text-white mb-6">Contact Form Submissions</h1>
            <div class="bg-gray-800 p-6 rounded-lg border border-gray-700 text-center">
                <p class="text-gray-400">No submissions yet.</p>
            </div>
        </div>
    }
}
