# Final Draft keyboard-shortcut reference

- **Captured:** 2026-11-06
- **Source:** [What keyboard shortcuts can I use in Final Draft?](https://kb.finaldraft.com/hc/en-us/articles/27977488282644-What-keyboard-shortcuts-can-I-use-in-Final-Draft)
- **Source title:** What keyboard shortcuts can I use in Final Draft?
- **Capture method:** Browser inspection of the article's headings, paragraphs, lists, and table cells.
- **Purpose:** Durable reference for ScriptCraft's hotkey and command-layer planning. This is not a claim that ScriptCraft implements the listed behavior.
- **Notation:** `Command` means the macOS Command key; `Control`, `Option`, `Alt`, `Shift`, and `Fn` are literal modifiers. `Primary` is introduced only in the implementation plan to mean Command on macOS and Control on Windows.
- **Transcription policy:** Action names and key combinations are preserved. Whitespace around `+` is normalized. Parenthetical source annotations are retained. Duplicate rows in different source sections remain visible. No omitted Windows Save As shortcut or other missing binding is invented.

## Return/Enter paragraph defaults

The source says that when the insertion point is at the end of a paragraph, Return/Enter inserts a paragraph whose kind depends on the paragraph it follows.

| From | To |
|---|---|
| General | General |
| Scene Heading | Action |
| Action | Action |
| Character | Dialogue |
| Dialogue | Action |
| Parenthetical | Dialogue |
| Transition | Scene Heading |
| Shot | Action |
| Cast List | Action |
| New Act | Scene Heading |
| End of Act | New Act |
| Summary | Summary |

The source describes these as defaults configurable through Format > Elements and the Next Element Enter/Return setting.

## Tab defaults and context-specific behavior

| From | To or result |
|---|---|
| Scene Heading | Action |
| Action | Character |
| Character | Transition, only when Character is blank |
| Transition | Scene Heading |
| Dialogue | Parenthetical |
| Parenthetical | Dialogue |
| Outline 1 | Outline 2 |
| Outline 2 | Outline 3 |
| Outline 3 | Note |
| Note | Note |

Additional source-described behavior:

| Context | Tab result |
|---|---|
| Beat Board, Beat title field | Move to Beat body field |
| Beat Board, Beat body field | Move to Beat title field |
| SmartType list | Accept the current selection and move to the next logical element |
| Scene Heading, Scene Intro such as INT or EXT | Add a period plus a space and move to Location; the next Tab calls up Location SmartType |
| Scene Heading, Location | Add space-dash-space and move to the time area; the next Tab calls up Time SmartType |
| Index Cards > Summary | Move from editable Scene Heading to editable Scene Summary to the next scene card's editable Scene Heading |
| Index Cards > Script | Move from editable Scene Heading to the next scene card's editable Scene Heading |
| Scene View | Move from editable Scene Heading to editable Scene Title to the next scene row's editable Scene Heading |

The source also says that typing `int.` or `ext.` at the beginning of an Action paragraph automatically creates a Scene Heading beginning with `INT.` or `EXT.` respectively.

## Mac element shortcuts

| Element | Shortcut |
|---|---|
| General | Command + 0 |
| Scene Heading | Command + 1 |
| Action | Command + 2 |
| Character | Command + 3 |
| Parenthetical | Command + 4 |
| Dialogue | Command + 5 |
| Transition | Command + 6 |
| Shot | Command + 7 |
| Cast List | Command + 8 |
| New Act | Command + 9 |
| Sequence | Command + Control + 8 |
| End of Act | Command + Control + 9 |
| Summary | Command + Control + 0 |
| Outline 1 | Command + Control + 1 |
| Outline 2 | Command + Control + 2 |
| Outline 3 | Command + Control + 3 |
| Note | Command + Control + 4 |

The source says these defaults may be changed in the Elements settings through the Shortcut key field.

## Mac keyboard shortcuts

### Final Draft 13 application

| Action | Shortcut |
|---|---|
| Hide Final Draft 13 | Command + H |
| Hide Others | Command + Option + H |
| Quit Final Draft 13 | Command + Q |

### File

| Action | Shortcut |
|---|---|
| New Document | Command + N |
| New from Template | Command + Shift + N |
| Open | Command + O |
| Close | Command + W |
| Save | Command + S |
| Save As | Command + Shift + S |
| Save As PDF | Command + Shift + P |
| Print | Command + P |

### Edit

| Action | Shortcut |
|---|---|
| Undo | Command + Z |
| Redo | Command + Shift + Z |
| Cut | Command + X |
| Copy | Command + C |
| Paste | Command + V |
| Select All | Command + A |
| Select Current Scene | Command + Shift + A |
| Next Change | Command + Option + K |
| Previous Change | Command + Option + Shift + K |
| Accept Change | Command + Option + A |
| Reject Change | Command + Option + R |
| Accept All Changes | Command + Option + Shift + A |
| Reject All Changes | Command + Option + Shift + R |
| Find | Command + F |
| Go To | Command + G |
| Find Selection | Command + Shift + F |
| Start Dictation | Fn + D |
| Emoji and Symbols | Fn + E |

### View

| Action | Shortcut |
|---|---|
| Hide / Show Outline Editor | Command + Shift + M |
| Hide / Show Outline in Script | Command + Shift + O |
| Show Invisibles | Command + J |
| Zoom In | Command + Plus Sign |
| Zoom Out | Command + Minus Sign |
| Show All Tabs | Command + Shift + Backslash |
| Enter Full Screen | Command + Option + F |

### Format

| Action | Shortcut |
|---|---|
| Open Elements Settings Window | Command + E |
| Remove Style from Selected Text | Command + T |
| Bold Selected Text | Command + B |
| Italicize Selected Text | Command + I |
| Underscore Selected Text | Command + U |
| Strike Through Selected Text | Command + Shift + X |
| All Caps | Command + K |
| Dual Dialogue | Command + D |
| Edit Dual Dialogue | Command + Shift + D |

### Insert

| Action | Shortcut |
|---|---|
| Add Alt Dialogue | Command + Shift + Period |
| Remove Alt Dialogue | Command + Shift + Comma |
| New Beat, Beat Board only | Command + Return |
| New Scene, Index Cards / Scene View only | Command + Shift + I |
| Insert Bookmark | Command + Backslash |
| Insert ScriptNote | Command + Shift + K |
| Insert Label | Command + Shift + L |
| Send to Script, Index Cards only | Command + Shift + C |

### Document

| Action | Shortcut |
|---|---|
| Open SmartType Window | Command + L |

### Tools

| Action | Shortcut |
|---|---|
| Spelling | Command + Comma |
| Thesaurus | Command + Semi-Colon |
| Open Reformat Tool | Command + R |

The source prints the Thesaurus key as `:` while naming it Semi-Colon. This likely describes the shifted punctuation key, but the table above deliberately preserves the source's action/key description rather than silently converting it to Command + Shift + Semicolon.

### Production

| Action | Shortcut |
|---|---|
| Edit Scene Number | Command + Shift + J |
| Mark Selected Text Revised | Command + Right Bracket |
| Mark Selected Text Unrevised | Command + Left Bracket |
| Tags Mode On / Off | Command + Shift + T |

### Window

| Action | Shortcut |
|---|---|
| Minimize | Command + M |
| Show Previous Tab | Control + Shift + Tab |
| Show Next Tab | Control + Tab |

### Mac element Tab/Return examples

| From | To | Press |
|---|---|---|
| Scene Heading Part 1 | Scene Heading Part 2 | Tab |
| Scene Heading Part 2 | Scene Heading Part 3 | Tab |
| Scene Heading Part 3 | Action | Return |
| Scene Heading Part 2 | Action | Return |
| Action | Action | Return |
| Action | Character | Return + Tab |
| Action | Shot | Control + 7 |
| Action | Transition | Control + 6 |
| Action | Scene Heading | Type int. or ext. |
| Character | Dialogue | Return |
| Character | Parenthetical | Tab |
| Parenthetical | Dialogue | Return |
| Dialogue | Action | Return |
| Dialogue | Character | Return + Tab |
| Dialogue | Scene Heading | Command + 1 |
| Dialogue | Transition | Command + 6 |
| Dialogue | Shot | Command + 7 |

### Mac special keys

| Key | Result |
|---|---|
| Page Up / Down | Page up or down one screen-full of text |
| Home | Move cursor to beginning of the line |
| End | Move cursor to end of the line |
| Command + Home | Move cursor to beginning of the document |
| Command + End | Move cursor to end of the document |
| Command + Right Arrow | Move cursor to beginning of next word |
| Command + Left Arrow | Move cursor to beginning of previous word |
| Up Arrow | Move cursor up a line |
| Down Arrow | Move cursor down a line |
| Option + Up Arrow | Move cursor up a paragraph |
| Option + Down Arrow | Move cursor down a paragraph |
| Shift + Right or Left Arrow | Extend highlighted selection one letter at a time |
| Command + Shift + Right Arrow | Extend highlighted selection one word at a time |
| Command + Shift + Left Arrow | Extend highlighted selection one word at a time |

## Windows element shortcuts

| Element | Shortcut |
|---|---|
| General | Control + 0 |
| Scene Heading | Control + 1 |
| Action | Control + 2 |
| Character | Control + 3 |
| Parenthetical | Control + 4 |
| Dialogue | Control + 5 |
| Transition | Control + 6 |
| Shot | Control + 7 |
| Cast List | Control + 8 |
| New Act | Control + 9 |
| Sequence | Control + Shift + 8 |
| End of Act | Control + Shift + 9 |
| Summary | Control + Shift + 0 |
| Outline 1, Acts | Control + Shift + 1 |
| Outline 2, Sequences | Control + Shift + 2 |
| Outline 3, Scenes | Control + Shift + 3 |
| Note | Control + Shift + 4 |

The source says these defaults may be changed in the Elements settings through the Shortcut key field.

## Windows keyboard shortcuts

### Home

| Action | Shortcut |
|---|---|
| Spelling | Control + D |
| Thesaurus | Control + Shift + D |
| Paste | Control + V |
| Cut | Control + X |
| Copy | Control + C |
| Bold Selected Text | Control + B |
| Italicize Selected Text | Control + I |
| Underscore Selected Text | Control + U |
| New ScriptNote | Control + J |
| Elements Settings | Control + E |
| Dual Dialogue | Control + Alt + D |

### File

| Action | Shortcut |
|---|---|
| New | Control + N |
| New from Template | Control + Shift + N |
| Open | Control + O |
| Close | Control + F4 |
| Save | Control + S |
| Save As PDF | Control + Shift + P |
| Print | Control + P |

The article does not list an ordinary Windows Save As shortcut in this table.

### Edit

| Action | Shortcut |
|---|---|
| Select All | Control + A |
| Select Current Scene | Control + Shift + A |
| Paste | Control + V |
| Copy | Control + C |
| Cut | Control + X |
| Undo | Control + Z |
| Redo | Control + Shift + Z |
| Find | Control + F |
| Go To | Control + G |
| Find Selection | Control + E |
| Accept Selected Change | Control + Alt + A |
| Reject Selected Change | Control + Alt + R |
| Go to Previous Change | Control + Shift + Alt + K |
| Go to Next Change | Control + Alt + K |

### View

| Action | Shortcut |
|---|---|
| Show / Hide Outline in Script | Control + Shift + O |
| Show / Hide Ruler | Control + Shift + H |

### Format

| Action | Shortcut |
|---|---|
| Dual Dialogue | Control + Alt + D |
| Elements Settings | Control + Shift + E |
| Grid Snapping | Control + Shift + Backslash |

### Insert

| Action | Shortcut |
|---|---|
| Insert ScriptNote | Control + J |
| Insert Bookmark | Control + M |
| Insert Alt Dialogue | Control + Alt + L |
| Remove Alt Dialogue | Control + Shift + Alt + L |
| Insert Beat, Beat Board only | Control + Enter |
| New Scene, Cards / Scene View only | Control + Shift + I |
| Send to Script, Cards / Scene View only | Control + Shift + C |

### Document

| Action | Shortcut |
|---|---|
| SmartType | Control + Shift + L |

### Tools

| Action | Shortcut |
|---|---|
| Spelling | Control + D |
| Thesaurus | Control + Shift + D |

### Production

| Action | Shortcut |
|---|---|
| Mark Selected Text Revised | Control + R |
| Mark Selected Text Unrevised | Control + Shift + R |
| Tags Mode On / Off | Control + Shift + T |

### Help

| Action | Shortcut |
|---|---|
| Final Draft User Guide | F1 |

### Beat Board

| Action | Shortcut |
|---|---|
| Zoom In, Beat Board only | Control + Plus |
| Zoom Out, Beat Board only | Control + Minus |

### Windows element Tab/Enter examples

| From | To | Press |
|---|---|---|
| Scene Heading Part 1 | Scene Heading Part 2 | Tab |
| Scene Heading Part 2 | Scene Heading Part 3 | Tab |
| Scene Heading Part 3 | Action | Enter |
| Scene Heading Part 2 | Action | Enter |
| Action | Action | Enter |
| Action | Character | Enter + Tab |
| Action | Shot | Control + 7 |
| Action | Transition | Control + 6 |
| Action | Scene Heading | Type int. or ext. |
| Character | Dialogue | Enter |
| Character | Parenthetical | Tab |
| Parenthetical | Dialogue | Enter |
| Dialogue | Action | Enter |
| Dialogue | Character | Enter + Tab |
| Dialogue | Scene Heading | Control + 1 |
| Dialogue | Transition | Control + 6 |
| Dialogue | Shot | Control + 7 |

### Windows special keys

| Key | Result |
|---|---|
| Page Up / Down | Page up or down one screen-full of text |
| Home | Move cursor to beginning of the line |
| End | Move cursor to end of the line |
| Control + Home | Move cursor to beginning of the document |
| Control + End | Move cursor to end of the document |
| Control + Right Arrow | Move cursor to beginning of next word |
| Control + Left Arrow | Move cursor to beginning of previous word |
| Up Arrow | Move cursor up a line |
| Down Arrow | Move cursor down a line |
| Alt + Up Arrow | Move cursor up a paragraph |
| Alt + Down Arrow | Move cursor down a paragraph |
| Shift + Right or Left Arrow | Extend highlighted selection one letter at a time |
| Control + Shift + Right Arrow | Extend highlighted selection one word at a time |
| Control + Shift + Left Arrow | Extend highlighted selection one word at a time |

## Reference conflicts and interpretation boundaries

1. The general Tab list says blank Character changes to Transition, while both platform example tables include Character to Parenthetical with Tab. ScriptCraft's current approved policy remains blank Character to Transition and nonblank Character as a consumed no-op; this reference capture does not authorize a semantic change.
2. The general Tab list says Note to Note. ScriptCraft's current approved matrix and shared formatting defaults say Note to Summary. The discrepancy is retained for review, not silently implemented.
3. The Mac example table lists Action to Shot and Action to Transition with Control-only shortcuts, while the Mac element table assigns Command + 7 and Command + 6. The requested ScriptCraft element shortcuts use the dedicated Mac element table as their authority.
4. Windows Home assigns Elements Settings to Control + E, Windows Format assigns it to Control + Shift + E, and Windows Edit assigns Find Selection to Control + E. These are source conflicts, not additional ScriptCraft commands.
5. Paste, Cut, Copy, Spelling, Thesaurus, and Dual Dialogue appear in more than one Windows section. They are duplicate presentations of bindings, not distinct command semantics.
6. Scene Heading Part 1/2/3, SmartType, Beat Board, Index Cards, Scene View, revision changes, Dual Dialogue, and other unavailable ScriptCraft features remain reference-only. Their presence here does not add functionality to the initial implementation.
7. Return/Enter and Tab behavior in ScriptCraft must continue to use its existing transactional core policy, including strict boundaries, Parenthetical handling, same-role no-op rules, and source-preservation safeguards. The reference does not supersede those durable requirements.
