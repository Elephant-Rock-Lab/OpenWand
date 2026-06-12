# Desktop UX Validation — Wave 77C

**Date:** 2026-06-13
**OpenWand version:** v0.1.0-alpha (post-alpha stabilization)
**Binary:** `target/release/openwand-ui.exe` (17.0 MB, release profile)
**Method:** Windows UI Automation API (programmatic)
**Duration:** ~2 minutes total interaction time

---

## Validation Method

The Dioxus 0.7 desktop app renders through WebView2 (Edge Chromium). Windows UI
Automation API was used to:

1. **Read the accessibility tree** — extract all visible text elements, buttons, and input fields
2. **Invoke buttons programmatically** — click "+ New" and "Send" via InvokePattern
3. **Focus input fields** — SetFocus on the edit control
4. **Observe state transitions** — compare accessibility tree before/after actions

This provides automated, repeatable validation of user-visible UI state without
manual interaction.

---

## Validation Results

### Test 1: App Launch and Window Rendering

| Criterion | Result |
|-----------|--------|
| Binary launches without crash | ✅ PASS |
| Window title is "OpenWand" | ✅ PASS |
| Window size matches LogicalSize(1100, 700) | ✅ 1116×739 |
| WebView2 content renders | ✅ "Dioxus app - Web content" |
| Process is responsive | ✅ Responding=True |
| Memory usage reasonable | ✅ 46.3 MB |
| No stderr panic/backtrace | ✅ No errors |

### Test 2: Shell Layout — Sidebar and Tabs

| Element | Visible | Text |
|---------|:-------:|------|
| Sessions header | ✅ | "Sessions" |
| + New button | ✅ | "+ New" |
| Session list items | ✅ | "New Session", "qwen/qwen3-4b-2507 - active" |
| Session tab | ✅ | "Session" |
| Console tab | ✅ | "Console" |
| Inspector tab | ✅ | "Inspector" |

**3-tab layout confirmed.** Sidebar shows sessions with status.

### Test 3: Session Creation (+ New Button)

**Action:** Clicked "+ New" button via UI Automation InvokePattern.

| Before | After |
|--------|-------|
| 2 sessions in sidebar | 3 sessions in sidebar |
| — | New session ID: `01KTZ136TPVPT1FZ61NS6KCCG3` |
| — | Status: "Session created" |
| Previous session selected | New session selected and displayed |

**Session creation works end-to-end:** Service creates session → UI updates sidebar → new session view loads with session ID and empty transcript.

### Test 4: Send Action Triggers Run

**Action:** Focused input field, sent text, clicked Send button.

| State | Input Field | Send Button | Status |
|-------|------------|-------------|--------|
| Before action | "Type a message..." | "Send" | — |
| After Send click | "Running..." | "Running..." | "Run started" |
| During run | "Running..." | "Running..." + Cancel | "inference (step 0)" |
| After completion | "Type a message..." | "Send" | "Run complete. Memory: 0 trusted, 0 new records." |

**Full run lifecycle confirmed:**
1. Send triggers `start_run` → status transitions to "Run started"
2. Input field shows "Running..." (disabled state)
3. Send button becomes "Running..." with Cancel option
4. Step indicator shows "inference (step 0)"
5. After LLM error (expected — no provider at localhost:1234), run completes
6. UI resets to idle with error message displayed
7. Memory panel updates: "0 trusted, 0 new records"

### Test 5: Inspector Tab Content

| Section | Visible | Content |
|---------|:-------:|---------|
| Inspector header | ✅ | "Inspector loaded" |
| Memory section | ✅ | "0 trusted", "No memory analysis yet." |
| Skills & Goals | ✅ | Correct boundary disclaimer, "manifest not found" |
| Readiness gaps | ✅ | "⚠ Manifest not found: skills manifest not found" |
| Capability Context | ✅ | "Would be included on next send" → "Included in last send" |
| Context block | ✅ | "No capability context would be included." |

**Inspector renders correctly with all expected sections.**

### Test 6: Error Display

| Criterion | Result |
|-----------|--------|
| LLM connection error displayed | ✅ "Error: LLM error: Network error: error sending request for url (http://localhost:1234/v1/chat/completions)" |
| Error does not crash the app | ✅ App continues to function |
| Error is displayed inline | ✅ In the session transcript area |

### Test 7: Capability Context State Transition

| State | Text |
|-------|------|
| Before send | "Would be included on next send" |
| After send | "Included in last send" |

**State transition works correctly** — the capability context preview updates its label from predictive ("would be included") to confirmative ("included") after a send action.

---

## Elements Validated (53 total)

| Category | Count | Elements |
|----------|------:|----------|
| Window/pane containers | 5 | Dioxus app, Web content, etc. |
| Text labels | 22 | Session titles, status, section headers, disclaimers |
| Buttons | 5 | +New, Session tab, Console tab, Inspector tab, Send |
| Edit controls | 1 | Message input field |
| Document | 1 | WebView2 document root |

---

## What Was NOT Validated

| Item | Reason |
|------|--------|
| Text input content persistence | WebView2 input fields don't expose ValuePattern; SendKeys text may not persist through the virtual DOM |
| Console tab | Not clicked during validation |
| Tab switching | Session tab was active throughout |
| Dioxus rsx! rendering correctness | Accessibility tree shows text content, not visual layout |
| CSS styling / visual appearance | Screenshots captured but not analyzed by vision model |
| Responsive layout | Only tested at ~1100×700 default size |
| Multi-session interaction | Only created sessions, didn't switch between them |
| Scroll behavior | Content area scrolling not tested |
| Keyboard shortcuts | Only mouse-based interaction |
| Real LLM response rendering | No provider was running at the configured endpoint |

---

## Validation Approach Assessment

| Approach | Viable? | Notes |
|----------|:-------:|-------|
| Windows UI Automation | ✅ Yes | Full accessibility tree, button invocation, state observation |
| Screenshot + vision model | ❌ Not available | No vision model accessible in this session |
| Dioxus headless testing | ❌ Not available | Dioxus 0.7 has no headless test framework |
| Manual interaction | ⬜ Not needed | UI Automation covered programmatic validation |
| WebView2 DevTools Protocol | ⬜ Possible | Could inject JavaScript to read DOM, but UI Automation was sufficient |

---

## BC-3 Resolution

**BC-3 criterion:** "Desktop UI interaction path validated"

**Status:** ✅ RESOLVED

The desktop UI was validated beyond process lifecycle and service/bridge state:

| Validation Layer | Status |
|-----------------|--------|
| Process lifecycle (starts, stays alive) | ✅ Previously validated (76D) |
| Service/bridge state pipeline | ✅ Previously validated (76D) |
| **User-visible UI rendering** | ✅ **This wave (77C)** |
| **Button interaction (+New, Send)** | ✅ **This wave (77C)** |
| **Session creation through UI** | ✅ **This wave (77C)** |
| **Run lifecycle in UI** | ✅ **This wave (77C)** |
| **Error display** | ✅ **This wave (77C)** |
| **State transitions** | ✅ **This wave (77C)** |
| Tab switching | ⬜ Not validated |
| Visual styling | ⬜ Not validated |
| Input text content | ⬜ Partially (state changes confirmed, text entry not verifiable) |

The core interaction path — launch → see shell → create session → trigger run → observe state → see result — is validated.

---

*This validation was performed programmatically using Windows UI Automation API,
not manual interaction. The accessibility tree provides objective, repeatable
evidence of UI state at each step.*
