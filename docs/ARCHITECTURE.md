# Himalayas Browser: Architecture

## System Overview

```
             Himalayas Headless Browser

+-----------------------------------------------------+

        Agent OS Layer (Permissions & Memory)
  
  - Permission Engine (scoped, time-bound)
  - Task Memory (ephemeral, isolated)
  - User Memory (persistent, encrypted)
  - Policy Engine (compliance, risk-based)
  - Audit & Observability (OTel)

   Headless Browser Runtime (Capability Provider)

  - Headless Chromium/WebKit (default, no rendering)
  - Semantic DOM Builder
  - Network Proxy & Intelligence
  - Session Manager
  - State Snapshots (for replay)
  - Optional Frontends (desktop, web, mobile, CLI)

        Tool Runtime Layer

  - HTTP APIs
  - Code Execution (sandboxed)
  - File System (scoped)
  - Database (query sandboxing)
  - External Services
  - IoT/Robotics (ROS2)

        Security & Isolation Layer

  - Sandbox (process, filesystem, network)
  - Identity & Secrets Management
  - Audit Logs (immutable)
  - Network Security

+-----------------------------------------------------+
```

## Core Design Principles

### 1. Agent Runtime ≠ Browser

The **browser is a capability provider**, not the agent itself.

```
Traditional View (Wrong):
Agent → Browser (controls everything)

Himalayas View (Right):
Agent → Permissions → Browser (reads DOM)
      ↓
      Tool Runtime (APIs, code, files)
      ↓
      Results
```

**Why**: Separates concerns. Agent controls what to do. Browser handles how to do it.

### 2. Permission-Scoped Ephemeral Sessions

Every agent task receives a scoped, time-bound permission envelope.

**Example: "Book my flight"**

```yaml
task_id: flight-booking-001
agent_id: travel-agent-v1

permissions:
  browser:
    domains:
      - airline.com
    actions: [navigate, query, submit_form]
    
  storage:
    read:
      - passport.pdf
      - preferences.json
    write: []
    
  payment:
    confirmation_required: true
    max_amount: 5000
    
  system:
    clipboard: false
    download: false

expires_at: 2026-08-06T10:30:00Z
timeout: 30_minutes

session:
  cookies: isolated
  localstorage: isolated
  memory: task-scoped
```

**Lifecycle**:
1. Agent receives permission envelope
2. Agent executes task (browser enforces constraints)
3. Task completes OR timeout
4. **Destroy session**
   - Revoke tokens
   - Clear cookies
   - Wipe memory
   - Delete temporary files
   - Log completion

### 3. Agent Memory Isolation

Three memory tiers, never mixed:

#### Task Memory (Ephemeral)
- Created when task starts
- Destroyed when task ends
- Not persisted
- Example: booking details during task

#### Session Memory (Short-term)
- Lives during browser session (hours/days)
- Cleared on session end
- Example: DOM state, navigation history

#### User Memory (Persistent)
- Survives sessions
- Encrypted at rest
- Example: preferences, credentials, history

**Enforcement**:
```rust
// Task memory cannot access user memory directly
agent.task_memory.get("booking") // ✓ OK
agent.task_memory.get_user_data() // ✗ Blocked

// Tasks can read user preferences, but cannot modify
agent.read_preference("preferred_airline") // ✓ OK
agent.write_user_memory("card_number", "...") // ✗ Blocked
```

### 4. Browser Context Snapshots

Before an agent acts, capture full state for audit and replay.

**Snapshot Contents**:
```rust
pub struct ContextSnapshot {
    pub timestamp: DateTime,
    pub url: String,
    pub dom_tree: SemanticDOM,
    pub cookies: HashMap<String, String>,
    pub localstorage: HashMap<String, String>,
    pub network_state: NetworkState,
    pub open_tabs: Vec<TabState>,
    pub permissions: PermissionSet,
    pub js_state: JSExecutionState,
}
```

**Usage**:

```
Agent Action: click("Buy Now")
       ↓
Snapshot + Action → Replay Engine
       ↓
Execute action in same state
       ↓
Verify result matches expected
       ↓
Audit log: [state_hash, action, result_hash]
```

**Benefits**:
- Full debugging capability
- Reproducibility (deterministic actions)
- Audit trail (what agent saw)
- Rollback (revert to previous state)

### 5. DOM-First Agent Interaction

Browser exposes **semantic APIs**, not screenshot-based vision.

**Anti-pattern** (fragile, vision-based):
```python
screenshot = browser.take_screenshot()
coords = vision_model.find_button(screenshot)
browser.click(coords)
```

**Pattern** (robust, intent-based):
```python
browser.find(
    intent="cancel subscription",
    role="destructive_action"
)
# Returns:
# {
#   button_id: "cancel-sub",
#   label: "Cancel Subscription",
#   requires_confirmation: true,
#   permission: "account:modify"
# }

browser.execute(action="cancel")
```

**Query API**:
```rust
pub enum Query {
    Button { label: &str, role: Option<Role> },
    Form { purpose: &str },
    Link { text: &str },
    Input { type_: &str, name: &str },
    Intent { intent: &str },
}

pub struct SemanticElement {
    pub id: String,
    pub role: Role,
    pub label: String,
    pub required_permission: Permission,
    pub state: ElementState,
}
```

### 6. Multi-Agent Architecture

Support multiple isolated agents in one browser runtime.

```
                 Supervisor Agent
                        │
        ┌───────────────┼───────────────┐
        │               │               │
   Research Agent  Shopping Agent   Coding Agent
        │               │               │
    Tab Group       Tab Group      Tab Group
        │               │               │
    Sandbox         Sandbox         Sandbox
        │               │               │
     Cookies        Cookies         Cookies
    (isolated)      (isolated)      (isolated)
        │               │               │
        └───────────────┼───────────────┘
                        │
                 Browser Runtime
```

**Isolation Guarantees**:
- Each agent gets separate tabs
- Cookies and storage isolated per agent
- Permissions scoped per agent
- Memory completely isolated
- No cross-agent communication (except via supervisor)

### 7. Built-in Observability (OTel)

Comprehensive telemetry at three levels:

**Agent Telemetry**:
```
span: agent.task
  - task_id
  - agent_id
  - status
  - duration
  - tokens_consumed
  - tool_calls
  - memory_used
  - permissions_used
  - errors
```

**Browser Telemetry**:
```
span: browser.navigate
  - url
  - status_code
  - latency
  - dom_size
  - resources_loaded
  - javascript_errors

span: browser.query
  - query_type
  - latency
  - results_count
```

**Security Telemetry**:
```
span: security.permission_check
  - permission_requested
  - granted
  - duration
  - risk_score

span: security.anomaly
  - anomaly_type
  - severity
  - context
```

**Export to**: Datadog, Prometheus, Honeycomb, Splunk, NewRelic, Jaeger

### 8. Network Intelligence Layer

Intelligent proxy between agent and internet.

```
Agent
  ↓
Browser Runtime
  ↓
Network Intelligence Layer
  ├─ API Detector (find REST/GraphQL endpoints)
  ├─ Dependency Mapper (trace API calls)
  ├─ Security Scanner (detect malware, trackers)
  ├─ Performance Monitor (latency, bandwidth)
  └─ Rate Limiter (respect site limits)
  ↓
Network Security
  ├─ Certificate Validation
  ├─ TLS Enforcement
  ├─ DNS Security
  └─ SOCKS/HTTP Proxy
  ↓
Internet
```

**Capabilities**:
- Detect GraphQL endpoints and schemas
- Map internal API dependencies
- Identify tracking pixels and cookies
- Detect suspicious network behavior
- Enforce rate limits per domain
- Proxy through corporate/VPN networks

### 9. Sandboxed Execution Model

Every agent execution is sandboxed.

**Sandbox Layers**:

1. **Process Sandbox** (OS-level)
   - Separate process per agent task
   - No access to host processes
   - Resource limits (CPU, memory)

2. **Filesystem Sandbox**
   - Separate temp directory per task
   - No access to sensitive paths (/etc, ~/.ssh)
   - Scoped read/write permissions

3. **Network Sandbox**
   - Proxy all network through security layer
   - Block connections to localhost (except approved)
   - Rate limiting per domain
   - Certificate pinning support

4. **Capability Sandbox**
   - Only use permitted APIs
   - Permissions validated on every call
   - Audit log every capability use

**Example**:
```rust
// Sandbox enforces this:
agent.browser.navigate("https://example.com") // ✓ Allowed
agent.browser.navigate("https://bank.com") // ✗ Blocked (not in permission envelope)
agent.file.read("/etc/passwd") // ✗ Blocked (no filesystem permission)
agent.spawn_process("curl ...") // ✗ Blocked (no code execution permission)
```

### 10. Agent Replay & Audit

Full replay capability for every agent execution.

**Example**:

```
User asks: "Why did my agent buy this?"

Browser provides replay:
10:32:01 Agent initialized
         Permission: airline.com, $5000 budget
         
10:32:05 Navigated to airline.com
         Snapshot A: homepage
         
10:32:15 Queried flights
         Query: cheapest flights NYC→SFO, Aug 10-12
         Results: 3 options, $200-400
         
10:32:25 Analyzed prices
         Decision: $280 flight meets criteria
         
10:32:40 Requested approval
         Approval: User confirmed
         
10:33:00 Submitted purchase
         Snapshot B: checkout page
         Form filled with: email, payment method
         Action: submit_form("checkout")
         
10:33:05 Purchase complete
         Snapshot C: confirmation page
         Order ID: ORD-123456
```

**How It Works**:

1. **Snapshot + Action Log**: Browser logs [snapshot, action, result] tuples
2. **Deterministic Replay**: Same input → same output (when possible)
3. **Audit Trail**: Full provenance of decisions
4. **Rollback**: Revert to previous state (if needed)
5. **Analysis**: Why did X happen?

**Storage**: 
- Encrypted in task database
- Indexed by task_id, agent_id, timestamp
- Retention: 90 days (configurable)
- Export format: JSON + HAR (HTTP Archive)

---

## Module Structure

```
src/
├── main.rs                  # Entry point, CLI, logging setup
│
├── daemon/
│   ├── mod.rs              # Config, Daemon struct, lifecycle
│   ├── lifecycle.rs        # Start, stop, restart, graceful shutdown
│   └── server.rs           # HTTP server, health endpoints
│
├── browser/
│   ├── mod.rs              # Browser trait and manager
│   ├── navigator.rs        # Page loading, redirects, cookies
│   ├── parser.rs           # HTML parsing
│   ├── semantics.rs        # Semantic DOM building
│   ├── snapshots.rs        # Context snapshots
│   └── storage.rs          # Session-scoped storage
│
├── agent/
│   ├── mod.rs              # Agent registry and lifecycle
│   ├── executor.rs         # Agent task execution
│   ├── memory.rs           # Memory isolation (task/session/user)
│   └── replay.rs           # Replay engine for audit
│
├── permission/
│   ├── mod.rs              # Permission engine
│   ├── request.rs          # Permission request handling
│   ├── expiration.rs       # Time-bound cleanup
│   └── audit.rs            # Permission audit log
│
├── network/
│   ├── mod.rs              # Network intelligence layer
│   ├── proxy.rs            # HTTP/HTTPS proxy
│   ├── detector.rs         # API endpoint detection
│   ├── mapper.rs           # Dependency mapping
│   └── security.rs         # Certificate, TLS, DNS
│
├── sandbox/
│   ├── mod.rs              # Sandbox container
│   ├── process.rs          # Process isolation
│   ├── filesystem.rs       # Filesystem restrictions
│   ├── network.rs          # Network rules
│   └── capabilities.rs     # Capability enforcement
│
├── observability/
│   ├── mod.rs              # OTel setup
│   ├── tracing.rs          # Distributed tracing
│   ├── metrics.rs          # Metrics collection
│   ├── logs.rs             # Structured logging
│   └── exporters.rs        # OTEL exporters
│
└── lib.rs                   # Public API exports
```

---

## API Surface (High-level)

### Agent APIs

```rust
pub trait AgentAPI {
    // Navigation
    async fn navigate(&self, url: &str) -> Result<()>;
    async fn go_back(&self) -> Result<()>;
    async fn go_forward(&self) -> Result<()>;
    
    // DOM Querying
    async fn find(&self, query: Query) -> Result<SemanticElement>;
    async fn find_all(&self, query: Query) -> Result<Vec<SemanticElement>>;
    
    // Interaction
    async fn click(&self, element_id: &str) -> Result<()>;
    async fn input(&self, element_id: &str, text: &str) -> Result<()>;
    async fn submit_form(&self, form_id: &str) -> Result<()>;
    
    // Inspection
    async fn get_dom(&self) -> Result<SemanticDOM>;
    async fn get_text(&self, element_id: &str) -> Result<String>;
    async fn get_attributes(&self, element_id: &str) -> Result<Map<String, String>>;
    
    // Code Execution
    async fn execute_js(&self, script: &str) -> Result<serde_json::Value>;
    async fn execute_js_on_element(&self, element_id: &str, script: &str) -> Result<Value>;
    
    // Memory
    async fn get_memory(&self, key: &str) -> Result<serde_json::Value>;
    async fn set_memory(&self, key: &str, value: Value) -> Result<()>;
    
    // Permission Checks
    async fn check_permission(&self, perm: Permission) -> Result<bool>;
    
    // Replay & Audit
    async fn get_replay(&self) -> Result<ReplayLog>;
}
```

### Permission APIs

```rust
pub trait PermissionEngine {
    async fn request(&self, perm: Permission, reason: &str) -> Result<PermissionGrant>;
    async fn check(&self, perm: Permission) -> Result<bool>;
    async fn revoke(&self, perm: Permission) -> Result<()>;
    async fn list_active(&self) -> Result<Vec<PermissionGrant>>;
}
```

---

## Data Flow Example: "Find and book cheapest flight"

```
1. User → Supervisor Agent
   "Book cheapest flight NYC→SFO, Aug 10-12, <$500"

2. Supervisor → Travel Agent (with ephemeral permissions)
   Permissions: airline.com, payment ($500), 30-min expiration

3. Travel Agent → Browser
   Action 1: navigate("https://airline.com")
   Snapshot 1: [URL, DOM, cookies]
   
4. Travel Agent → Browser
   Query: find_form(intent="flight_search")
   Result: {form_id, fields: [origin, destination, dates]}
   
5. Travel Agent → Browser
   Action 2: input("origin", "NYC")
   Snapshot 2: [URL, DOM, cookies]
   
6. Travel Agent → Browser
   Action 3: input("destination", "SFO")
   Snapshot 3: [URL, DOM, cookies]
   
7. Travel Agent → Browser
   Action 4: submit_form("flight_search")
   Snapshot 4: [URL, DOM results]
   
8. Travel Agent → Browser
   Query: find_all(role="flight_option")
   Result: [{price: 200}, {price: 280}, {price: 400}]
   
9. Travel Agent → Supervisor
   "Found 3 flights. Cheapest is $200 (meets $500 budget).
    Asking approval to book."
   
10. Supervisor → User
    "Found flight for $200. Approve?"
    
11. User → Supervisor
    "Approve"
    
12. Supervisor → Travel Agent
    Permission: approval_granted
    
13. Travel Agent → Browser
    Action 5: click("book_flight_200")
    Snapshot 5: [checkout page]
    
14. Travel Agent → Browser
    Action 6: input("email", user_email)
    Snapshot 6: [filled form]
    
15. Travel Agent → Browser
    Action 7: input("payment", payment_token)
    Snapshot 7: [complete form]
    
16. Travel Agent → Browser
    Action 8: submit_form("checkout")
    Snapshot 8: [confirmation page]
    
17. Travel Agent → Supervisor
    "Booking complete. Confirmation: ORD-123456"
    
18. Browser
    - Revoke permissions
    - Clear cookies
    - Wipe memory
    - Log all [snapshot, action, result] tuples
    - Audit trail complete

19. User → Browser
    "Why did you book this flight?"
    
20. Browser → Replay Engine
    Plays through all 8 snapshots + actions
    Shows complete decision chain
```

---

## Security Model

**Zero-Trust by Default**:
- No permission is implicit
- All permissions time-bound
- All actions audited
- All memory isolated
- All network proxied
- All processes sandboxed

**Permission Model**:
- Browser: domains, actions (navigate, query, interact)
- Storage: scoped read/write
- Payment: max amount, confirmation required
- System: clipboard, downloads, code execution
- Network: rate limits, IP ranges
- Secrets: credential access

**Audit Scope**:
- Every permission request
- Every permission grant
- Every permission use
- Every action executed
- Every network call
- Every error or anomaly

---

## Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Daemon startup | <500ms | Headless, metrics setup |
| Page load (semantic DOM) | <2s | vs 15-20s for visual rendering |
| Agent latency (navigate + query) | <500ms | vs 5-10s for screenshot+vision |
| Memory per agent | <100MB | vs 500MB+ per browser tab |
| Maximum concurrent agents | 1000+ | on 16GB RAM, single machine |
| Snapshot overhead | <5ms | serialize DOM + state |
| Permission check latency | <1ms | cached enforcement |

---

## Next Steps

1. **Phase 0** (Weeks 1-3): Daemon, health, metrics
2. **Phase 1** (Weeks 4-13): Browser + Agent APIs + Permissions
3. **Phase 2** (Weeks 14-28): Security hardening, studio, multi-agent
4. **Phase 3** (Weeks 29-39): Observability, network intelligence, replay

See [ROADMAP.md](./ROADMAP.md) for detailed timeline.
