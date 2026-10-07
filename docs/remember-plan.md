# Atlas: remembering what matters

Research and delivery plan · 2026-10-07

## Outcome and audience

For people who already write personal notes, turn a passage into an intention they can return to with its context, without maintaining another database. The first audience is existing Atlas users who write about people, ideas, and unfinished follow-ups. This is a product hypothesis, not demonstrated market demand. The supplied X posts provide qualitative prompts, not representative user research.

## Evidence and implications

- Gilbert et al., *Outsourcing Memory to External Tools* (2023; published online 2022), reviews intention offloading: people use external reminders to support delayed intentions, with choices affected by effort and metacognitive judgments. Implication: reduce capture effort and make the reason for resurfacing visible. This does not establish that Atlas reduces mental load in everyday life. https://doi.org/10.3758/s13423-022-02139-4
- Apple Reminders already offers dates, recurrence, location, people-triggered reminders, notes and images. Competing on generic task features alone is weak differentiation. https://support.apple.com/guide/reminders/add-or-change-reminders-remndc729e28/7.0/mac/26
- Bear documents a Shortcut that copies tasks into Reminders with a link to their original note. Source-linked reminders are an established interaction, not a new invention. Atlas can test whether capturing and revisiting context within the notebook feels simpler. https://blog.bear.app/2022/03/automate-your-notes-with-shortcuts-and-bear/
- Obsidian documents typed note properties, including dates. Portable text metadata is an established foundation. Atlas should keep the readable intention, date, person and source in Markdown rather than hiding the only copy in application state. https://obsidian.md/help/properties

Sources consulted 2026-10-07. Competitor assessment is based on documentation, not a hands-on comparative usability study. A prospective-memory experiment found through PMC could not be fetched reliably and is not used as evidence here.

## Decisions

Considered: (1) a separate task dashboard with projects and priorities; (2) automatic extraction and a proactive agent; (3) a small explicit remember-and-review loop attached to writing. Choose 3: closest to existing behavior, reversible, and testable without accounts, remote processing or speculative personality inference. Preserve washi, restrained controls, and selectable fountain typography. Use text labels for unfamiliar actions.

## Release 1 — implement now

1. Select a passage while editing, or enter context manually from a note. Choose Remember this.
2. Confirm a short intention, an optional person, and an optional review date. Display dates explicitly as YYYY-MM-DD. No inferred dates or unconfirmed extraction.
3. Save a separate ordinary Markdown note with a source wiki link and a captured quote. The original note is not rewritten. All metadata travels with notebook export.
4. For today shows open dated items due today or earlier. Undated items have a separate Anytime view; future and closed items remain accessible in All.
5. Show the exact review date/reason, person, source and captured context. Let the person complete, dismiss, reopen or reschedule an item. Keep closed records rather than deleting them.
6. Preserve existing save/conflict protection. Malformed metadata stays readable as a normal note. Renamed source links follow the existing wiki-link rewrite; missing sources are identified without inventing new notes.
7. In-app review only. The application must be opened to see due items; this is not a background notification service.

## File contract

Each remembered item is a root-level `Remember - … .md` note, with a version marker, title, status, optional ISO local date and optional person, a source wiki link and a blockquoted context snapshot. Parsing is strict. State updates replace only the intended metadata field and use Vault's conflict-aware atomic save; extra prose survives. The quote is historical context, not a promise to track future changes verbatim. Person is a user-entered label, not a profile or inferred trait.

## Validation and acceptance

Automated: Unicode passage capture; date/leap-year validation; open/today/past/future/undated/closed categorization; completion/reopen/reschedule; export/reopen without hidden state; rename and missing sources; stale-write refusal; preservation of additional prose; malformed records. Native: capture, review, navigate source, reschedule, close/reopen, narrow and standard layouts in disposable notebooks. Record unverified interactions explicitly. Existing zoom and refresh changes remain intact.

## Pilot before expanding

Invite 5–8 volunteers explicitly; no outreach is authorized by this plan. Run a 7-day pilot with synthetic examples first and participants' own private notebooks thereafter. Ask for 3–5 real intentions, not sensitive details. Use interviews or voluntary aggregate counts, with no automatic telemetry.

Proposed decision gates (targets, not observed results): at least 4 of 5 participants capture and find an item without help; typical capture under 30 seconds; at least 3 of 5 describe one useful resurfacing; zero lost drafts or overwritten external edits. Ask: what stopped occupying your head, what resurfaced at the wrong time, and what felt like extra administration? If it adds work, simplify before adding automation.

## Follow-on releases

2. After pilot: person filtering/timelines, a date picker, quick snooze, and opt-in system notifications with explicit delivery limits, permission handling, duplicate suppression and restart/time-zone tests.
3. Optional suggestions: candidates with exact source passages, user confirmation, correction/dismissal and a clear processing choice. No automatic messages, bookings or purchases. Decide local versus remote processing only with explicit user requirements and measured quality.
4. External services: optional calendar/reminder integration only after conflict, ownership and permission rules are specified. No requirement to leave Markdown or create an Atlas account.

Attachment viewing, tables/math and rich-text editing remain a separate notebook-quality backlog. This release prioritizes validating the remember-and-review loop.
