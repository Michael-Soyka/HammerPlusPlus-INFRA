# Addresses

In the tables below, slot `N` is the method index in the vtable, so the call looks like `call [eax + N*4]`.
VA is the function address in the INFRA DLL (image base `0x10000000`).

## How it works

Hammer++ for Portal 2 expects Portal 2 interfaces, and INFRA's differ from them in a few methods.
So Hammer++ gets a go-between:

- `infra_hpp_adapter.dll` (32-bit). The launcher loads it instead of `hammerplusplus_dll.dll`.
  The adapter loads the real DLL, hooks `IHammer::Connect` and hands Hammer++ proxies: their vtables
  follow the Portal 2 order, and each call goes on to the matching INFRA method.
  INFRA's own `steam_api.dll` is too old for Hammer++, so the adapter also takes `SteamClient017`
  straight from the Steam client's `steamclient.dll`.
  If something goes wrong, look at `bin\hammerplusplus\bin\infra_hpp_adapter.log`.
- `infra-hpp-patcher.exe` copies Hammer++ into `infra\bin`, points the launcher at the adapter
  and remembers the installed files, so that **Remove** can take them back.

## Unchanged

`ICvar`, `IFileSystem`, `IInputSystem`, `IStudioRender`, `IMDLCache`, `IDataCache`, vgui
(`IVGui`, `IInput`, `IPanel`, `ISchemeManager`, `ISystem`), `ILocalize`, physics,
`IMaterial`, `ITexture`, `IMaterialVar`.

These interfaces match Portal 2 and are handed to Hammer++ as they are.

`hammerplusplus_dll.dll` itself stays untouched on disk, `infra_hpp_adapter.dll` just sits next to it
in `bin\hammerplusplus\bin\`.

## `hammerplusplus.exe`

| VA | File offset | Before | After |
|---|---|---|---|
| `0x42845c` | `0x26e5c` | `hammerplusplus/bin/hammerplusplus_dll.dll` | `hammerplusplus/bin/infra_hpp_adapter.dll` + `00` |

## `IMaterialSystem` - `materialsystem.dll`, vtable `0x100957e4`

| Method | Portal 2 | INFRA | INFRA VA |
|---|---|---|---|
| `ModInit` | 11 `+0x2c` | 11 `+0x2c` | `0x1000bbd0` |
| `ModShutdown` | 12 `+0x30` | 12 `+0x30` | `0x1000a040` |
| Portal 2 only | 17 | - | - |
| `OverrideConfig` | 20 `+0x50` | 19 `+0x4c` | `0x1000fcf0` |
| `GetCurrentConfigForVideoCard` | 21 `+0x54` | 20 `+0x50` | `0x1000aeb0` |
| `SetMode` | 30 `+0x78` | 29 `+0x74` | `0x1000df40` |
| `BeginFrame` | 41 `+0xa4` | 40 `+0xa0` | `0x1000d3e0` |
| `EndFrame` | 42 `+0xa8` | 41 `+0xa4` | `0x10011140` |
| `SwapBuffers` | 45 `+0xb4` | 44 `+0xb0` | `0x1000d5e0` |
| `EvictManagedResources` | 46 `+0xb8` | 45 `+0xb4` | `0x1000ac80` |
| `AddReleaseFunc` | 49 `+0xc4` | 48 `+0xc0` | `0x10010be0` |
| `AddRestoreFunc` | 51 `+0xcc` | 50 `+0xc8` | `0x10010c50` |
| - | 59 `+0xec` | 58 `+0xe8` | `0x1000b010` |
| - | 66 `+0x108` | 65 `+0x104` | `0x1000a0f0` |
| - | 77 `+0x134` | 76 `+0x130` | `0x1000f1a0` |
| - | 80 `+0x140` | 79 `+0x13c` | `0x10010e10` |
| `CreateMaterial` | 81 `+0x144` | 80 `+0x140` | `0x1000a960` |
| `FindMaterial` | 82 `+0x148` | 81 `+0x144` | `0x10017390` |
| `FindTexture` | 89 `+0x164` | 88 `+0x160` | `0x1000a9b0` |
| - | 91 `+0x16c` | 90 `+0x168` | `0x1000a930` |
| `BeginRenderTargetAllocation` | 92 `+0x170` | 91 `+0x16c` | `0x1000b250` |
| `EndRenderTargetAllocation` | 93 `+0x174` | 92 `+0x170` | `0x1000b270` |
| `GetRenderContext` | 113 `+0x1c4` | 112 `+0x1c0` | `0x1000bc00` |
| - | 122 `+0x1e8` | 121 `+0x1e4` | `0x1000ee40` |
| - | 128 `+0x200` | 127 `+0x1fc` | `0x1000ac40` |

## `IMatRenderContext` - `materialsystem.dll`, vtable `0x10097d34`

| Portal 2 | INFRA | INFRA VA |
|---|---|---|
| 0–113 | 0–113 | - |
| 114 (Portal 2 only) | - | - |
| 117 `+0x1d4` | 116 `+0x1d0` | `0x1002d720` |
| 119 `+0x1dc` | 118 `+0x1d8` | `0x1002d850` |
| 124 `+0x1f0` | 123 `+0x1ec` | `0x1002d800` |
| 125 `+0x1f4` | 124 `+0x1f0` | `0x1002d820` |
| 126 `+0x1f8` | 125 `+0x1f4` | `0x1002d840` |
| 127 `+0x1fc` | 126 `+0x1f8` | `0x1002d2f0` |
| 131 `+0x20c` | 130 `+0x208` | `0x1002fe80` |

## `ISurface` / `IMatSystemSurface` - `vguimatsurface.dll`, vtable `0x100bbecc`

| Method | Portal 2 | INFRA | INFRA VA |
|---|---|---|---|
| `CreateFont` | 69 `+0x114` | 69 `+0x114` | `0x100045d0` |
| `SetFontGlyphSet` (9 -> 11 args) | 70 `+0x118` | 70 `+0x118` | `0x100045e0` |
| `AddCustomFontFile` | 71 `+0x11c` | 71 `+0x11c` | `0x1000d7f0` |
| `GetFontTall` | 72 `+0x120` | 72 `+0x120` | `0x10004660` |
| INFRA only | - | 73 `+0x124` | `0x10004680` |
| `GetCharABCwide` | 75 `+0x12c` | 76 `+0x130` | `0x100046e0` |
| `GetCharacterWidth` | 76 `+0x130` | 77 `+0x134` | `0x10004710` |
| `GetTextSize` | 77 `+0x134` | 78 `+0x138` | `0x10004760` |
| `DestroyTextureID` | 141 `+0x234` | 142 `+0x238` | `0x10003cd0` |
| Portal 2 only | 151 | - | - |
| `EnableWindowsMessages` | 152 `+0x260` | 152 `+0x260` | `0x10003940` |
| `InstallPlaySoundFunc` | 158 `+0x278` | 158 `+0x278` | `0x10004eb0` |
| `DrawColoredText` | 160 `+0x280` | 160 `+0x280` | `0x1000b930` |

## Crash causes by address

Call sites in `hammerplusplus_dll.dll` (image base `0x10000000`) that crashed before the adapter.

| # | Hammer++ call | Cause | Fix |
|---|---|---|---|
| 1 | `0x10063635`: `[ecx+0x6c]` | INFRA's `steam_api.dll` (`SteamClient012`) has no such slot | `SteamClient` import redirected to `steamclient.dll` `SteamClient017` |
| 2 | `0x100865cd`: `[eax+0xb8]` | INFRA slot 46 is `ReleaseResources` | `IMaterialSystem` proxy |
| 3 | `0x10119cbd`: `[eax+0x118]` | `SetFontGlyphSet`: 9 args vs 11 in INFRA | adapter |
| 4 | `0x101233b6`: `[eax+0x1fc]` | INFRA method pops 16 extra bytes | `IMatRenderContext` proxy |
