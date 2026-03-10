# ACP UI Coverage

## Covered by the current UI

- User, assistant, and reasoning message chunks are streamed into `agent_chat`.
- ACP plans are mapped into the shared plan rail.
- ACP permission requests are mapped into the shared inline request UI.
- ACP tool calls and tool call updates are mapped into structured tool messages.
- ACP available commands are surfaced as a system note in chat.
- ACP current mode changes are surfaced as a system note in chat.
- ACP session config updates are surfaced as a system note in chat.
- ACP model and thought-level config selectors are exposed through the existing model / effort UI when the agent publishes ACP config options for those categories.

## Needs new UI for full ACP support

- Slash command discovery and execution UX.
  The current chat only shows available ACP commands as informational text. There is no command palette, autocomplete, or dedicated command input flow.

- Session mode controls.
  The current chat can show mode changes, but it cannot let the user switch ACP session modes.

- Session config controls beyond model / thought level.
  ACP can expose arbitrary config selectors. The current UI only has established controls for model and reasoning effort.

- Rich terminal tool content.
  ACP tool calls can embed terminals. The current tool message UI can only show a summarized terminal row, not an embedded live terminal.

- Rich non-text tool content.
  ACP tool calls can emit images, audio, and embedded resources. The current tool message UI only summarizes those payloads as text rows.

- Optional ACP extensions not enabled in this build.
  Session info updates and usage updates exist behind ACP crate feature flags, but this build does not currently surface them.
