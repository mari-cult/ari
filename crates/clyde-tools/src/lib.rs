/*fn main() {
    println!("Hello, world!");
}
use {
    indexmap::IndexMap, pyo3::types::{IntoPyDict, PyCFunction, PyFunction, PyList, PySequence, PyTuple, PyType}, pyo3_ext::{prelude::*, sync::LazyImport}, std::{
        collections::HashMap,
        env,
        fmt::{self, Write},
    }, twilight_cache_inmemory::DefaultInMemoryCache, twilight_gateway::{
        error::ReceiveMessageErrorType, Event, EventTypeFlags, Intents, Shard, ShardId, StreamExt,
    }, twilight_http::{response::TextFuture, Client}
};

macro_rules! langchain_tools {
    ($($tool:ident),*) => {{
        #[pymodule]
        fn langchain_tools(module: &Bound<'_, PyModule>) -> PyResult<()> {
            register_tools(module, [$(pyo3::wrap_pyfunction!($tool, module)?,)*])?;

            Ok(())
        }

        pyo3::append_to_inittab!(langchain_tools);
    }};
}

/// Load the specified URL, returning text, images, or videos.
#[pyfunction]
fn load_url(query: String) -> PyResult<String> {
    println!("load_url(query={query:#?})");

    Ok(String::from("404 Not Found"))
}

/// Draw an image of the specified prompt.
#[pyfunction]
fn draw_image(prompt: String) -> PyResult<String> {
    println!("draw_image(prompt={prompt:#?})");

    Ok(String::from("nigger"))
}

fn import_error(library: &str) -> ! {
    panic!("Failed to `import {library}`, perhaps you forgot to create or source venv, and `pip install -r requirements.txt`?")
}

fn langchain_tools_error() -> ! {
    panic!("Failed to `import langchain_tools`, did you forget `langchain_tools!(tool_a, tool_b, ...)`?")
}

pub trait ImportRequired {
    fn import_bound_required(&self, name: &str) -> Bound<'_, PyModule>;
}

impl ImportRequired for Python<'_> {
    fn import_bound_required(&self, name: &str) -> Bound<'_, PyModule> {
        self.import_bound(name)
            .unwrap_or_else(|_error| import_error(name))
    }
}

macro_rules! import {
    ($py:expr, $module:literal) => {
        ($py)
            .import_bound($module)
            .unwrap_or_else(|_error| import_error($module))
    };
}

static ISBUILTIN: LazyImport<PyFunction> = LazyImport::new("inspect", "isbuiltin");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    langchain_tools!(load_url, draw_image);
    pyo3::prepare_freethreaded_python();
    tracing_subscriber::fmt::init();

    tokio::spawn(async {
        tokio::signal::ctrl_c().await;
        std::process::exit(1);
    });

    let token = env::var("DISCORD_TOKEN")?;
    let cache = DefaultInMemoryCache::builder()
        .message_cache_size(50)
        .build();

    let intents = Intents::DIRECT_MESSAGES | Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT;
    let mut shard = Shard::new(ShardId::ONE, token.clone(), intents);
    let client = Client::new(token);

    while let Some(result) = shard.next_event(EventTypeFlags::all()).await {
        let event = match result {
            Ok(event) => event,
            Err(error) if is_fatal(error.kind()) => {
                tracing::error!("Fatal gateway error: {error}");

                return Err(error.into());
            }
            Err(error) => {
                tracing::warn!("Gateway error: {error}");

                continue;
            }
        };

        cache.update(&event);

        let (channel_id, guild_id) = match event {
            Event::MessageCreate(event) => (event.channel_id, event.guild_id),
            Event::MessageDelete(event) => (event.channel_id, event.guild_id),
            Event::MessageDeleteBulk(event) => (event.channel_id, event.guild_id),
            Event::MessageUpdate(event) => (event.channel_id, event.guild_id),
            _ => continue,
        };

        let Some(current_user) = cache.current_user() else {
            continue;
        };

        let Some(channel_messages) = cache.channel_messages(channel_id) else {
            continue;
        };

        let mut messages = Vec::new();

        for message_id in channel_messages.iter().rev() {
            let Some(message) = cache.message(*message_id) else {
                tracing::error!("Missing cache entry for message {message_id}.");

                continue;
            };

            let author_id = message.author();

            let Some(author) = cache.user(author_id) else {
                tracing::error!("Missing cache entry for user {author_id}.");

                continue;
            };

            if let Some(guild_id) = guild_id {
                let Some(member) = cache.member(guild_id, author_id) else {
                    tracing::error!(
                        "Missing cache entry for member {author_id} in guild {guild_id}."
                    );

                    continue;
                };
            }

            let role = if author.id == current_user.id {
                "assistant"
            } else {
                "user"
            };

            let name = author.name.as_str();
            let content = message.content();

            messages.push((role, format!("{name}: {content}")));
        }

        let content = Python::with_gil(|py| {
            let langchain_experimental_llms_ollama_functions =
                import!(py, "langchain_experimental.llms.ollama_functions");

            let sdkit = import!(py, "sdkit");

            let langchain_tools = py
                .import_bound("langchain_tools.generated")
                .unwrap_or_else(|_error| langchain_tools_error());

            let kwargs = [("model", "llama3")].into_py_dict_bound(py);

            let llm = langchain_experimental_llms_ollama_functions
                .getattr("OllamaFunctions")?
                .call((), Some(&kwargs))?;

            let llm = llm
                .getattr("bind_tools")?
                .call1((langchain_tools.getattr("list")?,))?;

            let output = llm.getattr("invoke")?.call1((messages,))?;
            let tool_calls = output.getattr("tool_calls")?;

            if tool_calls.is_empty().unwrap_or(false) {
                let content = output.getattr("content")?.extract::<String>()?;

                return Ok(content);
            }

            let structured_tool = py
                .import_bound("langchain_core.tools")?
                .getattr("StructuredTool")?
                .downcast_into::<PyType>()
                .map_err(PyErr::from)?;

            for (tool_name, tool_class) in langchain_tools.getmembers()? {
                tool_name
            }

            let tool_list = langchain_tools
                .getmembers()?
                .into_iter()
                .filter(|pair| {
                    pair.get_item(1)
                        .and_then(|tool| tool.is_instance(&structured_tool))
                        .unwrap_or(false)
                })
                .collect::<Vec<_>>();

            let tool_name = tool_calls.getattr("name")?;

            tool_list.get(tool_name)


            Ok::<_, anyhow::Error>(content)
        })?;

        if content.is_empty() {
            tracing::error!("no reply");

            continue;
        }

        client.create_message(channel_id).content(&content).await?;
    }

    Ok(())
}

fn is_fatal(error_type: &ReceiveMessageErrorType) -> bool {
    matches!(
        error_type,
        ReceiveMessageErrorType::Reconnect | ReceiveMessageErrorType::WebSocket
    )
}

pub struct PyTool {

}

pub struct PydanticTool<'a> {
    pub module: &'a str,
    pub name: &'a str,
    pub args: &'a str,
    pub output: &'a str,
    pub doc: &'a str,
}

impl<'a> fmt::Display for PydanticTool<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("@tool\ndef ")?;
        fmt.write_str(self.name)?;
        fmt.write_str(self.args)?;
        fmt.write_str(" -> ")?;
        fmt.write_str(self.output)?;
        fmt.write_str(":\n    \"\"\"")?;
        fmt.write_str(self.doc)?;
        fmt.write_str("\"\"\"\n    return ")?;
        fmt.write_str(self.module)?;
        fmt.write_char('.')?;
        fmt.write_str(self.name)?;
        fmt.write_str(self.args)?;
        fmt.write_str("\n\n")?;

        Ok(())
    }
}

fn register_tools<'py>(
    module: &Bound<'py, PyModule>,
    tools: impl IntoIterator<Item = Bound<'py, PyCFunction>>,
) -> PyResult<()> {
    let py = module.py();

    let mut generated = String::from("from langchain_core.tools import tool\n\n");
    let module_name = module.name()?;
    let module_name = module_name.extract()?;

    for tool in tools {
        let name = tool.name()?;
        let name = name.extract()?;
        let signature = tool.text_signature()?;
        let signature = signature.extract()?;
        let doc = tool.doc()?.expect("doc");
        let doc = doc.extract()?;

        let pydantic_tool = PydanticTool {
            module: module_name,
            name,
            args: signature,
            output: "str",
            doc,
        };

        write!(&mut generated, "{pydantic_tool}").unwrap();

        module.add_function(tool)?;
    }

    let structured_tool = py
        .import_bound("langchain_core.tools")?
        .getattr("StructuredTool")?
        .downcast_into::<PyType>()
        .map_err(PyErr::from)?;

    let generated_module =
        PyModule::from_code_bound(py, &generated, "generated.py", "langchain_tools.generated")?;

    let list = generated_module
        .getmembers()?
        .into_iter()
        .flat_map(|pair| pair.get_item(1))
        .filter(|tool| tool.is_instance(&structured_tool).unwrap_or(false))
        .collect::<Vec<_>>();

    println!("{list:#?}");

    let list = PyList::new_bound(py, list);

    generated_module.add("list", list)?;

    module.add_submodule(&generated_module)?;

    Ok(())
}


pub struct ToolBuilder {
    name: String,
    description: String,
}

impl Tool {
    pub fn builder() -> ToolBuilder {
        ToolBuilder {
            name
        }
    }
}

Tool::new("draw_image")
    .description("Draw an image of the specified prompt.")
    .arg(Arg::string("prompt"))
    .run(|prompt: String| async move {

    })


pub type ToolResult = std::result::Result<ToolOutput, ToolError>;

pub struct ToolOutput {

}

enum ToolErrorInner {
    Other(Box<dyn std::error::Error + Send + Sync>),
}

pub struct ToolError {
    inner: ToolErrorInner,
}

impl ToolError {
    #[inline]
    const fn new(inner: ToolErrorInner) -> Self {
        Self { inner }
    }

    pub fn other<E>(error: Box<E>) -> Self
    where
        E: Into<Box<std::error::Error + Send + Sync>>,
    {
        Self::new(error)
    }
}


pub mod sdkit {
    pub struct GenerateOptions {
        prompt: String,
    }

    impl GenerateOptions {
        pub fn new<P: Into<String>>(prompt: P) -> Self {
            let prompt = prompt.into();

            Self { prompt }
        }

        pub fn run(self) -> () {
            todo!()
        }
    }
}

async fn imagine(prompt: String) -> ToolResult {
    let image = sdkit::GenerateOptions::new(prompt).run().await?;

    ToolResult::image(image)
}

async fn calculator(input: String) -> ToolResult {
    ToolResult::text("5")
}

async fn load_url(url: String) -> TextResult {
    ToolResult::text("404 Not Found.")
}

/// A registry of tools.
#[derive(Default)]
pub struct ToolRegistry {
    registry: IndexMap<String, ToolData>,
}

impl ToolRegistry {
    /// Create an empty tool registry.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool by the specified name.
    pub fn register<N, D, R>(&mut self, name: N, description: D, run: R) -> ToolResult
    where
        N: Into<String>,
        D: Into<String>,
        R: Into<Box<dyn Future<Output = ToolResult> + Send + Sync>,
    {
        const NAME_RANGE: Range<usize> = 3..24;
        const DESCRIPTION_RANGE: Range<usize> = 10..32;

        fn is_lowercase_or_hyphen(character: char) -> bool {
            matches!(character, 'a'..='z' | '_')
        }

        let name = name.into();

        if NAME_RANGE.contains(&name.len()) && name.chars().all(is_lowercase_or_hyphen){
            return ToolResult::error("`name` must be non-empty, and 3 to 24 lowercase or hyphen characters long.");
        }

        if DESCRIPTION_RANGE.conrains(&description.len())
    }
}

tool_manager.register("imagine", "Draw an image of the specified prompt", imagine);
tool_manager.register("calculator", "Calculate the provided input", imagine);
*/

pub use self::error::ToolError;

mod error;

pub(crate) mod pydantic;

pub mod registry;
pub mod validate;

pub type ToolResult<T> = std::result::Result<T, ToolError>;
