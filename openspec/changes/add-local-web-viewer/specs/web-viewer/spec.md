# Spec Delta

## Purpose

Defines a read-only local web viewer that lets the person running Tala see the conversations, agents and outstanding work in a Tala home, without taking part in them.

## ADDED Requirements

### Requirement: Viewer command serves the current home
`tala ui` SHALL serve the viewer for the Tala home selected by `TALA_HOME`, or the default home, until interrupted. It SHALL write the viewer URL to stdout as the command's data in the selected `--format`, write logs to stderr, open the URL in the default browser unless `--no-open` is given, and exit 0 when interrupted. A browser that fails to open SHALL NOT stop the server.

#### Scenario: Start the viewer
- **WHEN** a person runs `tala ui` in a home with a store
- **THEN** stdout reports a loopback URL and the home path, and the server stays running until interrupted

#### Scenario: Do not open a browser
- **WHEN** a person runs `tala ui --no-open`
- **THEN** the URL is reported and no browser is launched

#### Scenario: Text output
- **WHEN** a person runs `tala ui --format=text`
- **THEN** stdout contains the URL as plain text

### Requirement: Viewer is independent of the daemon
The viewer SHALL read the home's store directly through a read-only connection. It SHALL NOT start, stop, adopt or send requests to the daemon, take daemon or startup locks, or write to the store or any other file in the home. It SHALL work whether or not the daemon is running.

#### Scenario: Daemon stopped
- **WHEN** the daemon for the home is not running and a person runs `tala ui`
- **THEN** the viewer serves the stored messages and no daemon is started

#### Scenario: Store unchanged
- **WHEN** the viewer has served every view
- **THEN** the store's contents and schema version are unchanged

### Requirement: Viewer refuses unusable homes
`tala ui` SHALL fail with a structured error and exit 1, without serving, when the home has no store, when the home is connected to a remote host, or when the store's schema version differs from the version this binary uses. Each error SHALL include a hint naming the corrective action.

#### Scenario: No store
- **WHEN** a person runs `tala ui` in a home that has never been used
- **THEN** the command fails with exit code 1 and a hint to use Tala in that home first

#### Scenario: Remote-connected home
- **WHEN** a person runs `tala ui` in a home connected with `tala network connect`
- **THEN** the command fails with `REMOTE_MODE` and a hint to run the viewer on the host

#### Scenario: Schema version mismatch
- **WHEN** the store's schema version is older or newer than the binary's
- **THEN** the command fails with exit code 1 and a hint that names the version mismatch and how to resolve it

### Requirement: Viewer is reachable only from this machine
The viewer SHALL listen only on the loopback interface, on the port given by `--port` or an operating-system-assigned port. It SHALL reject any request whose `Host` header is not a loopback name with the listening port.

#### Scenario: Loopback binding
- **WHEN** the viewer is running
- **THEN** it accepts connections on loopback and on no other interface

#### Scenario: Foreign host header
- **WHEN** a request arrives with a `Host` header naming another host
- **THEN** the viewer rejects it without returning data

### Requirement: Viewer data requires a per-run token
Each run SHALL generate a random access token and include it in the reported URL in a form that browsers do not send to the server or record in request logs. Every data request SHALL present the token, compared in constant time; a missing or wrong token SHALL be rejected without data. The page itself SHALL contain no message data.

#### Scenario: Missing token
- **WHEN** a client requests viewer data without the token
- **THEN** the viewer rejects the request and returns no data

#### Scenario: Token from an earlier run
- **WHEN** a client presents the token from a previous `tala ui` run
- **THEN** the viewer rejects the request

### Requirement: Viewer never exposes private handles
The viewer SHALL NOT read, return or render private handles, selector contents or handle-derived values. Agents SHALL be identified only by public ID, name, tool and registered context.

#### Scenario: Agent data
- **WHEN** the page requests agent data
- **THEN** the response contains no `th_` handle or stored handle value

### Requirement: Viewer page is self-contained
The page SHALL be served by the binary itself and SHALL NOT load scripts, styles, fonts or other resources from any other origin. Message text SHALL be rendered as text, never interpreted as markup.

#### Scenario: Markup in a message
- **WHEN** a message contains HTML or script markup
- **THEN** the viewer displays the markup as literal text

#### Scenario: Offline use
- **WHEN** the machine has no network access
- **THEN** the viewer loads and works fully

### Requirement: Feed view
The feed SHALL list messages newest first, grouped by day, showing sender, recipient or board, project, time, message text and request or delivery state. Long text SHALL be shortened with a way to expand it. It SHALL load a bounded number of messages and let the person load older ones.

#### Scenario: Direct request
- **WHEN** an agent has sent a request that has not been answered
- **THEN** the feed shows it from sender to recipient as awaiting a reply

#### Scenario: Board post
- **WHEN** an agent posts to a project board
- **THEN** the feed shows the post addressed to that project's board

#### Scenario: Older messages
- **WHEN** the store holds more messages than the first page and the person asks for older ones
- **THEN** the next older page is appended without repeating messages

### Requirement: Conversations view
The conversations view SHALL list threads by latest activity with their participants, message count, latest text and whether a reply is awaited. Selecting a thread SHALL show its full transcript in order.

#### Scenario: Open a thread
- **WHEN** the person selects a thread from the feed or the list
- **THEN** the full transcript of that thread is shown, oldest first

### Requirement: Agents view
The agents view SHALL list agents by most recent activity with tool, project, branch, whether each is listening now or was recently active, messages sent and received, unread messages and requests it owes a reply to. It SHALL also rank pairs of agents by the number of direct messages between them.

#### Scenario: Coordinator with workers
- **WHEN** one agent has exchanged messages with several others
- **THEN** each pair appears in the ranking with its direct message count

### Requirement: Attention view
The attention view SHALL list requests awaiting a reply and unread messages whose recipient was active within the last day, and separately list requests whose recipient has not been seen for over a day as abandoned. The attention count SHALL include only items whose recipient was active within the last day.

#### Scenario: Request to a departed agent
- **WHEN** a request is unanswered and its recipient was last seen two days ago
- **THEN** it appears under abandoned requests and not in the attention count

#### Scenario: Unread message to an active agent
- **WHEN** an agent seen in the last hour has an unread message
- **THEN** the message appears under unread with whether that agent is listening

### Requirement: Shared filters
Every view SHALL apply the same project, agent and text filters. Selecting an agent's name in any view SHALL set the agent filter.

#### Scenario: Filter by agent
- **WHEN** the person filters by one agent
- **THEN** every view shows only messages that agent sent or received

### Requirement: Live updates
While the page is open, the viewer SHALL show new messages, state changes and presence changes within a few seconds without a manual reload, and SHALL indicate when it has lost contact with the server.

#### Scenario: New message arrives
- **WHEN** an agent sends a message while the page is open
- **THEN** the message appears within a few seconds

#### Scenario: Server stopped
- **WHEN** the `tala ui` process exits while the page is open
- **THEN** the page indicates that it is disconnected
