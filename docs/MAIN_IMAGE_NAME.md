# The project name of the main image

The recovery image (scrivo-safe) accepts an upload for `ota_0` only when the project name inside the image
(`esp_app_desc_t.project_name`) is the word it was built to expect. The name of this tree's images is the build setting
`SCRIVO_MAIN_PROJECT`, and it is the same value as `SCRIVO_SAFE_MAIN_PROJECT` of the recovery image, set on two sides
of one handshake: change one and change the other, or the recovery image refuses the upload.

- Not set, or empty: `micropython`, which is also the default of `SCRIVO_SAFE_MAIN_PROJECT`.
- Set: the value, at most 31 printable ASCII characters without spaces. A longer or other name stops the build.
- Read by the build script of the device (`crates/version`, `emit_app_version`), handed to the image as
  `APP_PROJECT_NAME`; Cargo rebuilds the image when the variable changes.

```sh
SCRIVO_MAIN_PROJECT=scrivo-main tools/build.sh bench_s3     # the image says "Project name: scrivo-main"
```

The recovery image's own name is set in its `menuconfig` (`SCRIVO_SAFE_MAIN_PROJECT`), see its README.
