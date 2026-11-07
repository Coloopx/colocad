# Native SOLIDWORKS author (experimental)

The Windows entry point consumes the same job. It starts a blank part using the application's configured default, builds sketches with native lines and circles, calls extrusion/cut/fillet/pattern APIs and names the resulting features. The fillet uses body edges and the native pattern selects the original seed feature. There are no part-specific model files. Running it requires local Windows, .NET 8 and a licensed SOLIDWORKS installation with a valid default blank part configuration.

```powershell
colocad solidworks .\manifold\part\native-job.json .\manifold.SLDPRT
```

Use `COLOCAD_SW_INTEROP` for an alternate directory containing the local SOLIDWORKS interop DLLs. The completion check is designed to save a temporary part, reopen it, verify native types, radius, pattern seed, count, pitch and reference volume, then edit a driving extrusion dimension, rebuild, restore and check it. Only a successful check publishes the output. The current release has **no native runtime pass** for either complex part. The package includes no four-hole template and does not route jobs to another machine.


[Validation notes](validation.md) · [API](api.md)
