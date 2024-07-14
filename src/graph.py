#import clyde_bridge

from langchain_core.messages import AIMessage, HumanMessage, SystemMessage
from langchain_core.tools import tool
from langchain_experimental.llms.ollama_functions import OllamaFunctions
from langgraph.checkpoint import MemorySaver
from langgraph.graph import END, MessagesState, StateGraph
from langgraph.prebuilt import ToolNode
from typing import Annotated, Literal, TypedDict

@tool
def search(query: str):
    """Surf the web"""

    return clyde_bridge.bridged_search(query)

@tool
def imagine(prompt: str):
    """Imagine a stable diffusion prompt"""

    return clyde_bridge.bridged_imagine(prompt)

builder = StateGraph(MessagesState)
tools = [search, imagine]
model = OllamaFunctions(model="gemma2", format="json").bind_tools(tools)

def should_continue(state: MessagesState) -> Literal["tools", END]:
    messages = state["messages"]
    last_message = messages[-1]

    if not isinstance(last_message, AIMessage):
        return END

    if last_message.tool_calls:
        return "tools"
    
    return END

def call_model(state: MessagesState):
    global model
    
    messages = state["messages"]
    response = model.invoke(messages)

    print(response)
    
    return {"messages": [response]}

builder.add_node("clyde", call_model)
#builder.add_node("tools", ToolNode(tools))

builder.set_entry_point("clyde")
#builder.add_conditional_edges("clyde", should_continue)

#builder.add_edge("tools", "clyde")

graph = builder.compile(checkpointer=MemorySaver())
