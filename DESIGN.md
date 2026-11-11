# SkillBridge OSS design system

## 1. Atmosphere and identity

Extend the existing classroom workspace. Deep green class headers, warm neutral backgrounds and white panels keep instructions and student work readable. The bridge mark identifies the app. This feature reuses the current presentation and form components.

## 2. Color

Existing tokens define the palette. Background is #f4f6f2 and text is #172d27. Panels use --paper (#fff), --line (#dce5df), --muted (#4b635a), --accent (#135c49), --accent-dark (#104a3b), and --wash (#e4f1e9). Status colors extend the existing green and gray badges with --warning-ink (#7a4610) and --warning-wash (#fff1d6) for revisions and lateness. Use neutral outlines for controls and tonal washes for selection.

## 3. Typography

Use the existing system sans-serif stack. Body text is 1rem with 1.55 line-height, form labels are .875rem, metadata is .78rem, and section headings are 1.125rem. Task headings reuse the section size. Display identifiers in the browser's monospace stack. Long instructions, URLs and feedback wrap within their panel.

## 4. Spacing and layout

Use --space for responsive page padding, --radius for panel corners, and --shadow for existing panel depth. Existing section gaps are 1rem and form gaps are .75rem. New components use --practice-gap (1rem), --practice-inset (.75rem), and --practice-small-gap (.5rem). The assignment list and detail use one column. Rubric label/point inputs use two columns, with the label taking the available width. At 820px and below, rubric rows stack.

## 5. Components

ActionForm owns submit loading, validation errors and reset behavior. Notice renders errors with role alert. Reuse panel, form-stack, primary and secondary controls, multiline text, section-heading and status badges. Assignment buttons show selection through a wash and aria-pressed. Submission cards identify the student, revision number and timestamp. Reviews display each criterion and score, feedback and outcome. Native details hides the teacher's task-creation form until requested.

## 6. Interaction and motion

Use native buttons, inputs, selects and details. Provide explicit submit actions and five-second data refresh. No decorative animation is introduced. Inputs are disabled while their form is saving. A reviewed attempt is read-only. Revision requests reveal a new student submission form. Switching assignment resets only the detail view for that assignment.

## 7. Accessibility and responsive constraints

Every input has an associated visible label. Rubric fields include criterion numbers in their labels. Keyboard focus uses the existing focus-visible outline. Outcome and lateness always include text. External result links display the destination and open with noopener and noreferrer. Forms and feedback remain usable without horizontal scrolling at 375, 768 and 1280 pixels.

## 8. Accepted debt and evidence

The existing UI supports Indonesian. Credential issuance is a separate planned module. Task rubrics are immutable in this iteration. Task lists show the newest 100 tasks and detail shows the most recent 200 submissions. Real browser checks must cover teacher creation and review, student submission and revision, accepted results, empty states and mobile layouts before publishing.
