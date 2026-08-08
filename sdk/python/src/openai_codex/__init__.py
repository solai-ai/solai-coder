"""Python SDK for running SolaiAgent workflows.

Start with :class:`SolaiAgent` for synchronous applications or
:class:`AsyncSolaiAgent` for async applications. Most programs create a thread and
run a turn::

    from openai_codex import SolaiAgent, Sandbox

    with SolaiAgent() as codex:
        thread = codex.thread_start(sandbox=Sandbox.workspace_write)
        result = thread.run("Describe this project.")
        print(result.final_response)
"""

from ._version import __version__
from .api import (
    ApprovalMode,
    AsyncChatgptLoginHandle,
    AsyncDeviceCodeLoginHandle,
    AsyncSolaiAgent,
    AsyncThread,
    AsyncTurnHandle,
    ChatgptLoginHandle,
    DeviceCodeLoginHandle,
    ImageInput,
    Input,
    InputItem,
    LocalImageInput,
    MentionInput,
    SolaiAgent,
    RunInput,
    Sandbox,
    SkillInput,
    TextInput,
    Thread,
    TurnHandle,
    TurnResult,
)
from .client import SolaiAgentConfig
from .errors import (
    InternalRpcError,
    InvalidParamsError,
    InvalidRequestError,
    JsonRpcError,
    MethodNotFoundError,
    SolaiAgentError,
    SolaiAgentRpcError,
    ParseError,
    RetryLimitExceededError,
    ServerBusyError,
    TransportClosedError,
    is_retryable_error,
)
from .retry import retry_on_overload

__all__ = [
    "__version__",
    "SolaiAgentConfig",
    "SolaiAgent",
    "AsyncSolaiAgent",
    "ApprovalMode",
    "Sandbox",
    "ChatgptLoginHandle",
    "DeviceCodeLoginHandle",
    "AsyncChatgptLoginHandle",
    "AsyncDeviceCodeLoginHandle",
    "Thread",
    "AsyncThread",
    "TurnHandle",
    "AsyncTurnHandle",
    "TurnResult",
    "Input",
    "InputItem",
    "RunInput",
    "TextInput",
    "ImageInput",
    "LocalImageInput",
    "SkillInput",
    "MentionInput",
    "retry_on_overload",
    "SolaiAgentError",
    "TransportClosedError",
    "JsonRpcError",
    "SolaiAgentRpcError",
    "ParseError",
    "InvalidRequestError",
    "MethodNotFoundError",
    "InvalidParamsError",
    "InternalRpcError",
    "ServerBusyError",
    "RetryLimitExceededError",
    "is_retryable_error",
]
