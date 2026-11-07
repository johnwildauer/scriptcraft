# Shared application icon resources

[`AppIcon.png`](AppIcon.png) is the single canonical artwork source shared by the runtime and platform packaging scripts. Update this PNG when the artwork changes; do not keep platform-local PNG copies.

[`AppIcon.ico`](AppIcon.ico) is a derived Windows resource, generated and maintained manually. Windows Cargo builds embed this pre-generated ICO into the executable for its Explorer/EXE icon. macOS does not use the ICO. The Windows portable package stages the canonical PNG, not a separate ICO file.

With ImageMagick installed, regenerate the ICO manually from the repository root when desired:

```powershell
magick packaging/resources/AppIcon.png -define icon:auto-resize=256,128,64,48,32,16 packaging/resources/AppIcon.ico
```

ImageMagick is only used when you manually choose to regenerate the ICO; Cargo does not run ImageMagick or generate the ICO. Review and commit the resulting ICO deliberately, preserve the PNG as the source of truth, and rebuild the Windows executable after changing the ICO for the embedded Explorer icon to update.
