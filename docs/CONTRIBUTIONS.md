# Contributing findings back

Drafts of what this launcher found that the wider community could use
(SDK `docs/playbooks/contribute-back.md`). **The owner reviews and posts;
agents only draft.** Nothing here has been posted. Post a draft only after
the owner approves it by number, then record the link and date in its
status. Before posting a PCGamingWiki draft, check the live page again,
fill in `{{Refcheck|user=…}}`, and check each hex edit on a copy of the
exact executable named (same SHA-256). Never attach game files.

Copied from the SDK's `docs/CONTRIBUTIONS.md` (prepared 2026-11-02), with
this repository's updates noted.

| # | Finding | Target | Status |
| --- | --- | --- | --- |
| R1 | stuken's SegaRallyRevoStutterFix fixes the 16/17 ms jitter and the Championship stage-2 stutter | PCGW *Sega Rally Revo*, Frame rate | **ready** |
| R2 | ultra-wide aspect-table offsets for the clean `SEGA Rally_SSE1.exe` v4.0.6.0 | PCGW Ultra-widescreen | needs verification (no 21:9 screen tested) |
| R3 | Sega Rally Revo under Wine: saved-game layout, Wine desktop on multi-monitor hosts, slow logo videos, DXVK on hybrid GPUs | PCGW Linux note | **ready**, with the update below |
| U1 | ask stuken to add a licence to SegaRallyRevoStutterFix | upstream issue | draft, owner to decide |

## Ready drafts

### R1 — Sega Rally Revo: name the stutter fix

Page: <https://www.pcgamingwiki.com/wiki/Sega_Rally_Revo>. The current
"Enable high frame rate" fixbox names no tool, and the only community
workaround elsewhere is a 0.96 Cheat Engine speedhack. Draft:

```wikitext
{{Fixbox|description=Use SegaRallyRevoStutterFix|ref=<ref>{{Refcheck|user=OWNER|date=2026-11-02|comment=Applied to SEGA Rally_SSE1.exe v4.0.6.0; result matches the fix's reference hash; races driven on Wine 11.20 + DXVK.}}</ref>|fix=
{{ii}} Fixes the frame drop/duplicate every ~26 frames (the race clock advances in whole milliseconds, 16/17 ms) and the extra stutter from stage 2 of Championship (a second pacer thread).
{{ii}} Only for the clean {{file|SEGA Rally_SSE1.exe}} (3,727,360 bytes, MD5 <code>4e71704338c6d7f3ef57ea81fe5b015d</code>); launch the game through that file afterwards.
# Download [https://github.com/stuken/SegaRallyRevoStutterFix SegaRallyRevoStutterFix] and follow its instructions.
}}
```

Notes: this is stuken's work. Our part is confirming that it applies to the
widely mirrored v4.0.6.0 SSE1 build (SHA-256
`22655a54bad6932239ade58325703bd4b5a2020376737748026443fa5aadf76e`) and
reproduces the reference result
(`4c08c1deae28a329b6588138198556ac8eaded8444d0eb94a1fe2f1ad08f2520`). In-race
frame pacing was not measured; the owner's race recording is the
gameplay evidence. Also mention, as `{{ii}}` or in the note, that DXVK's
`d3d9.maxFrameRate = 60` is the usual extra guard under Linux.
Martijnsx/Sega-Rally-Revo-Frame-Pacing-Fix targets a different build
(3,700,786 bytes) and should not be suggested for this one.

### R3 — Sega Rally Revo under Wine (PCGW Linux note)

The retail AppDB entry is rated Garbage (Wine 3.0.3). Our verified runs
used DXVK, so they are not AppDB material. A plain-Wine retest (no DXVK) is
needed for an AppDB report. Text for a PCGW Linux note (do not mention the
copy's origin, and describe the disc check only in general terms):

> Wine 11.20 (WoW64) + DXVK 3.1.1 d3d9: plays (races driven with an XInput
> pad). Needed:
> - the June 2010 DirectX runtime pieces `winetricks d3dx9 directplay`
>   (a main-menu access violation otherwise);
> - `Documents\SEGA Rally\Saved Games\config.ini` written by hand, because
>   the .NET `SEGARallyLauncher.exe` is unreliable under Wine;
> - the saved-game layout `Saved Games\SEGA Rally.dat` and
>   `Public Documents\SEGA Rally\Shared Data\Rank Table\SR_RANKTABLES.DAT`,
>   or the main menu crashes;
> - on multi-monitor hosts with a high-refresh monitor, a Wine virtual
>   desktop, because the game ignores `RunWindowed=1` and its exclusive
>   mode change fails.
>
> The startup logo videos (`Bootup/video/Rally/*.wmv`) take very long under
> Wine's GStreamer path, and renaming them skips them. On hybrid
> laptops/desktops DXVK may pick the iGPU and show black frames, so force
> the discrete GPU's Vulkan ICD.

Caveat: the saved-game crash may be specific to installs that never ran
the original installer/launcher. Phrase it as "if …".

**Update (2026-11-02, this repository):** the d3dx9/directplay bullet is
doubtful. The owner's first AppImage run and the recorded race used a fresh
prefix without winetricks and reached a race, so with Wine 11.20 the
built-in DLLs sufficed; the crash may have been the missing saved-game
layout. Drop that bullet, or phrase it as "if the main menu still crashes
after creating the saved-game files, install d3dx9 (winetricks)".

## After verification

### R2 — Sega Rally Revo ultra-wide, clean SSE1 build

PCGW sends ultra-wide players to Widescreen Fixer, and WSGF's method
searches `39 8E E3 3F`. For `SEGA Rally_SSE1.exe` v4.0.6.0 (SHA-256
`22655a54…a5aadf76e`), the two 16:9 slots are at file offsets **2F7C44**
and **2FADA8** (each `39 8E E3 3F`, flanked by 4:3 `AB AA AA 3F` and 16:10
`CD CC CC 3F`). Replace both with the target ratio: 21:9 (64/27 =
2560x1080) `26 B4 17 40`; 3440x1440 `8E E3 18 40`; 32:9 `39 8E 63 40`.
Needed: a 21:9 test. The reference host is 16:9 and the ultra-wide path
was only unit-tested. Do not post offsets for `SEGA Rally.exe` from our
copy: it is a NoDVD build.

The offsets above are now also this launcher's `patches::ASPECT_21_SSE1` /
`ASPECT_32_SSE1` tables (and `ASPECT_*_MAIN` for the NoDVD build, which is
never posted); `patch::describe` prints them.

## Upstream

### U1 — stuken/SegaRallyRevoStutterFix: a licence

The repository has no licence file, and this launcher reimplements the fix
with the same cave machine code. Draft issue (owner to decide whether to
post it, or to replace the caves with independently written code):

> **Title:** Licence for the stutter fix?
>
> Thanks for SegaRallyRevoStutterFix: it fixed both stutter bugs on the
> v4.0.6.0 SSE1 build for us. The Sega Rally Revo launcher
> (github.com/akitaonrails/rea-sega-rally-revo) applies the same fix to a
> private copy of the player's game (it reproduces your reference SHA-256
> `4c08c1de…f1ad08f2520`), and credits you. The repository has no licence
> yet; would you add one (MIT, for example) so others can reuse the patch
> bytes with clear terms? If you'd rather we didn't ship them, say so and
> we will change course.
