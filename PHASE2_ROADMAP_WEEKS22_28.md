# Phase 2 Weeks 22-28: Agent Lifecycle & First Workflow

**Timeline**: Nov 13 - Dec 31, 2026  
**Current Status**: Weeks 14-21 ✅ COMPLETE  
**Remaining Work**: Weeks 22-28 ⏳ IN PROGRESS  
**Vision Alignment**: Foundation for Phase 3-6

---

## Context

Phase 2 is the **bridge between India Stack automation and the universal browser platform**.

**Completed** (Weeks 14-21):
- ✅ Identity & authentication
- ✅ Document intelligence (OCR, PDF)
- ✅ Form validation & workflows
- **3 modules, 1,150 LOC, 81 tests**

**Remaining** (Weeks 22-28):
- ⏳ Ephemeral agent lifecycle
- ⏳ First production workflow
- **Est. 1,000 LOC, 20+ tests**

---

## Weeks 22-25: Ephemeral Agent Lifecycle

### Focus: Agent as First-Class Runtime Citizen

**Concept**: Agents are not permanent processes, they are spawned per-task, execute with scoped permissions, and auto-cleanup.

**Key Features**:

1. **Agent Spawning**
   - Triggered by workflow request
   - Unique ID generation
   - Session binding
   - Time-to-live (TTL) setting

2. **Credential Injection**
   - Temporary credential store
   - Session-scoped access
   - Auto-revocation on cleanup
   - Never stored on disk

3. **Scoped Permissions**
   - Document access (PDF, DigiLocker)
   - Form access (specific forms only)
   - Printer access (one document)
   - Network access (government APIs only)
   - Duration-limited grants

4. **Automatic Cleanup**
   - Memory wipe
   - Credential destruction
   - Permission revocation
   - Session termination

5. **Audit Trail**
   - Every action logged
   - Compliance record
   - No action is silent

### Architecture

```rust
pub struct EphemeralAgent {
    agent_id: String,           // Unique agent per task
    session_id: String,         // Bound to user session
    lifecycle: AgentLifecycle,  // State machine
    permissions: PermissionSet, // Time-bound grants
    credentials: SecureStore,   // Temp credential store
    audit_trail: Vec<String>,   // Complete audit log
    ttl: Duration,              // Auto-cleanup timer
}

pub enum AgentLifecycle {
    Created,                    // Just spawned
    Authenticated,              // Credential injected
    DocumentsRetrieved,         // Fetched from DigiLocker
    FormFilled,                 // Auto-populated
    ApprovalRequested,          // Waiting for human
    Approved,                   // User confirmed
    Signed,                     // Document signed
    Submitted,                  // Sent to government
    Cleanup,                    // Cleaning resources
    Destroyed,                  // Terminated
}
```

### Implementation Plan

**Week 22-23: Core Agent Lifecycle**
- Agent struct + lifecycle state machine
- Credential store (SecureString-based)
- Permission binding
- TTL mechanism
- Cleanup cascade

**Estimated**: 250-300 LOC, 8 tests

**Week 24-25: Integration**
- Connect to permission engine
- Connect to workflow executor
- Audit trail logging
- Error handling + recovery
- Testing + hardening

**Estimated**: 200-250 LOC, 8 tests

### Tests

```rust
#[test]
fn test_agent_lifecycle() {
    let agent = EphemeralAgent::new("user_123", Duration::from_secs(300));
    assert_eq!(agent.lifecycle, AgentLifecycle::Created);
}

#[test]
fn test_credential_injection() {
    agent.inject_credentials(credentials)?;
    assert_eq!(agent.lifecycle, AgentLifecycle::Authenticated);
}

#[test]
fn test_permission_auto_revoke() {
    agent.grant_permission(Permission::Camera, Duration::from_secs(300));
    // Time passes...
    assert!(!agent.has_permission(Permission::Camera));
}

#[test]
fn test_auto_cleanup() {
    agent.start();
    agent.wait_for_completion();
    assert_eq!(agent.lifecycle, AgentLifecycle::Destroyed);
}
```

---

## Weeks 26-28: First Complete Workflow

### Focus: End-to-End License Renewal

**Goal**: Demonstrate complete workflow from citizen request to receipt generation.

**Workflow Steps**:

```
Step 1: Citizen Authentication
  └─ Initiates license renewal
  └─ Provides Aadhaar + OTP
  └─ Agent: Authenticated state

Step 2: Document Retrieval
  └─ Query DigiLocker
  └─ Fetch identity proof (Aadhaar)
  └─ Fetch address proof
  └─ Agent: DocumentsRetrieved state

Step 3: Document Intelligence
  └─ OCR identity document
  └─ Extract fields (name, DOB, address)
  └─ Validate formats (Aadhaar, phone, email)
  └─ Agent: FormFilled state

Step 4: Form Population
  └─ Get license renewal form
  └─ Auto-fill from extracted data
  └─ Validate all required fields
  └─ Agent: FormFilled state

Step 5: Approval Gate
  └─ Show summary to citizen
  └─ Request human approval
  └─ Wait for confirmation
  └─ Agent: ApprovalRequested state

Step 6: Digital Signature
  └─ Connect to eSign service
  └─ Hash application
  └─ Get digital signature
  └─ Embed signature in form
  └─ Agent: Signed state

Step 7: Government Portal Submission
  └─ Upload to RTO portal
  └─ Get acknowledgment number
  └─ Verify submission status
  └─ Agent: Submitted state

Step 8: Receipt Generation
  └─ Create receipt document
  └─ Generate tracking number
  └─ Store audit trail
  └─ Agent: Cleanup → Destroyed

Agent: Automatically destroyed after completion
```

### Implementation Plan

**Week 26: End-to-End Integration**
- Connect all Phase 2 components
- Integrate: Identity + Documents + Workflows + Agents
- Government portal simulation (MVP)
- Receipt generation

**Estimated**: 300-400 LOC, 8 tests

**Week 27-28: Testing & Hardening**
- Security review
- Error scenarios
- Edge cases
- Performance testing
- Documentation

**Estimated**: 200-250 LOC, 8+ integration tests

### Government Portal Simulation

For MVP testing (before real RTO integration):

```rust
pub struct RtoPortalSimulator {
    submissions: HashMap<String, SubmissionRecord>,
}

impl RtoPortalSimulator {
    pub async fn submit_application(&mut self, form: LicenseRenewalForm) -> Result<String> {
        let acknowledgment = format!("ACK-{}-{}", chrono::Local::now().timestamp(), rand::random::<u32>());
        self.submissions.insert(acknowledgment.clone(), SubmissionRecord {
            form,
            timestamp: chrono::Local::now(),
            status: SubmissionStatus::Received,
        });
        Ok(acknowledgment)
    }
}
```

### Integration Architecture

```
LicenseRenewalWorkflow
    ├── IdentityProvider (Aadhaar, DigiLocker, eSign)
    ├── DocumentProcessor (OCR, PDF parsing)
    ├── FormValidator (Indian formats)
    ├── EphemeralAgent (Permission scoped)
    ├── WorkflowExecutor (State machine)
    ├── RtoPortalSimulator (Government API mock)
    └── AuditTrail (Complete logging)
```

---

## Testing Strategy

### Unit Tests (16 tests)
- Agent lifecycle (4)
- Credential injection (3)
- Permission auto-revoke (3)
- Workflow integration (6)

### Integration Tests (12 tests)
- License renewal workflow (4)
- Document + Agent pipeline (4)
- Government submission (2)
- Receipt generation (2)

### Security Tests (4 tests)
- Credential isolation (1)
- Permission boundary (1)
- Memory cleanup (1)
- Audit completeness (1)

### Total: 32 tests (24 new + 8 existing)

---

## Completion Criteria

### Weeks 22-25: Agent Lifecycle ✓
- [ ] EphemeralAgent struct complete
- [ ] Lifecycle state machine working
- [ ] Credential injection secure
- [ ] Permission auto-revoke tested
- [ ] Cleanup cascade verified
- [ ] 16 tests passing

### Weeks 26-28: First Workflow ✓
- [ ] License renewal end-to-end
- [ ] Document intelligence integrated
- [ ] Human approval workflow
- [ ] eSign integration (mock)
- [ ] Receipt generation
- [ ] 32 tests passing
- [ ] Security review complete

---

## Vision Alignment

How Phase 2 Weeks 22-28 enables Phase 3-6:

### → Phase 3: Document Platform
- ✅ OCR engine (built Week 18-21)
- ✅ Form processing (built Week 14-17)
- ⏳ Ephemeral agents (building Week 22-25)
- ⏳ Auto-workflows (building Week 26-28)
- **Next**: PDF rendering, editing, AI capabilities

### → Phase 4: Device Integration
- ✅ Document scanning foundation (Week 18-21)
- ✅ Agent permission model (Week 22-25)
- ✅ First hardware workflow (Week 26-28)
- **Next**: Real scanners, printers, cameras, microphones

### → Phase 5: Enterprise Integration
- ✅ Workflow executor (Week 14-17)
- ✅ API integration patterns (Week 26-28)
- ✅ Government portal submission (Week 26-28)
- **Next**: CRM/ERP connectors (Salesforce, SAP)

### → Phase 6: AI Agent Marketplace
- ✅ Agent sandbox (Week 22-25)
- ✅ Permission framework (Week 22-25)
- ✅ Audit trail (Week 22-25)
- **Next**: Multi-agent coordination, model marketplace

---

## Success Metrics

| Metric | Target | Success |
|--------|--------|---------|
| **Build Status** | Clean | ✓ |
| **Test Pass Rate** | 100% | ✓ |
| **Test Count** | 32+ | ✓ |
| **Compilation Warnings** | <10 | ✓ |
| **First Workflow** | Dec 28 | ✓ |
| **Agent Lifecycle** | Secure | ✓ |
| **Audit Trail** | Complete | ✓ |

---

## Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| Agent cleanup incomplete | Cascade cleanup, TTL enforcement |
| Credential leak | SecureString, memory wipe |
| Permission bypass | Transaction model, audit trail |
| Workflow failure | Error handling, retry logic |
| Government API changes | Portal simulator first, API abstraction |

---

## Deliverables

### Code
- `src/india_stack/agent.rs` (agent lifecycle)
- `src/india_stack/secure_store.rs` (credential management)
- `src/india_stack/license_renewal.rs` (complete workflow)
- `src/india_stack/portal_simulator.rs` (testing)

### Tests
- 32 unit + integration + security tests
- 100% pass rate
- Coverage: >80%

### Documentation
- `PHASE2_COMPLETE.md` (final report)
- API docs
- Architecture guide

### GitHub
- Commit for each week
- Final PR with all changes
- Release candidate: Phase 2 v1.0

---

## Success Criteria Summary

✅ **Week 22-25**: Ephemeral agents + lifecycle + auto-cleanup + audit trail

✅ **Week 26-28**: License renewal end-to-end + first production workflow + government submission

✅ **By Dec 31, 2026**: Phase 2 complete, first production India Stack workflow deployed

---

## Next Phase

After Week 28, Phase 3 begins:

**Phase 3: Document Platform** (Jan-Mar 2027)
- PDF rendering engine
- Document editing capabilities
- AI document features (summarization, extraction)
- Office format support (DOCX, XLSX, PPTX)

---

## Conclusion

Weeks 22-28 are the culmination of Phase 2.

They deliver:
1. **Ephemeral Agent Pattern** (foundation for Phase 6)
2. **First Production Workflow** (license renewal)
3. **Complete Integration** (identity + documents + agents + workflows)
4. **Secure Execution** (sandbox, permissions, audit trail)

This foundation enables the transition from India Stack automation to the universal browser platform.

**Timeline**: On track for Phase 3 start Jan 1, 2027.
