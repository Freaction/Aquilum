{
  lib,
  rustPlatform,
  pkg-config,
  makeWrapper,
  patchelf,
  fontconfig,
  freetype,
  gst_all_1,
  libx11,
  libxcb,
  libxcursor,
  libxi,
  libxrandr,
  libxkbcommon,
  wayland,
}:

let
  # Keep the Nix package version in sync with the Cargo workspace.
  version = (lib.importTOML ../aquilum-app/native/Cargo.toml).package.version;

  # Libraries that winit, softbuffer, arboard and fontique load lazily with
  # dlopen(3). They are not recorded as DT_NEEDED entries, so they have to be
  # added to the binary's RUNPATH explicitly.
  runtimeLibs = [
    fontconfig
    freetype
    libx11
    libxcb
    libxcursor
    libxi
    libxrandr
    libxkbcommon
    wayland
    gst_all_1.gstreamer
    gst_all_1.gst-plugins-base
  ];

  # Same set of plugins as the .deb/.rpm packages: playback, demuxers,
  # codecs. The app links GStreamer itself, but the plugins are loaded at
  # runtime by the GStreamer registry.
  gstreamerPlugins = with gst_all_1; [
    gst-plugins-base
    gst-plugins-good
    gst-plugins-bad
    gst-libav
  ];
in
rustPlatform.buildRustPackage {
  pname = "aquilum";
  inherit version;

  src = lib.cleanSource ../aquilum-app;

  # The workspace stores Cargo output in ../../.artifacts during normal
  # development. A Nix build must keep it inside the build directory instead.
  env.CARGO_TARGET_DIR = "target";

  # cargo setup hook
  cargoHash = "sha256-cm7OyLIQ0/JdvZ+8uNiVr9Wr2pdYrM2RgOWGrqnatYE=";

  # The release packages are built with the `installed` feature: it enables
  # auto-start and hides the development-only command line flags.
  buildFeatures = [ "installed" ];

  nativeBuildInputs = [
    pkg-config
    makeWrapper
    patchelf
  ];

  buildInputs = runtimeLibs;

  # Upstream tests are run by the project's own CI. They exercise the GUI and
  # are not a prerequisite for packaging the binary.
  doCheck = false;

  # `native/Cargo.toml` names the binary `aquilum-native`; the desktop entry,
  # the .deb/.rpm packages and the AppImage all call it `aquilum-app`.
  postInstall = ''
    mv "$out/bin/aquilum-native" "$out/bin/aquilum-app"

    install -Dm644 native/packaging/Aquilum.desktop \
      "$out/share/applications/Aquilum.desktop"

    install -Dm644 native/icons/32x32.png \
      "$out/share/icons/hicolor/32x32/apps/aquilum-app.png"
    install -Dm644 native/assets/icon-64.png \
      "$out/share/icons/hicolor/64x64/apps/aquilum-app.png"
    install -Dm644 native/icons/128x128.png \
      "$out/share/icons/hicolor/128x128/apps/aquilum-app.png"
    install -Dm644 native/icons/128x128@2x.png \
      "$out/share/icons/hicolor/256x256/apps/aquilum-app.png"
  '';

  postFixup = ''
    # Make lazily loaded X11/Wayland/fontconfig/GStreamer libraries resolvable.
    patchelf --add-rpath "${lib.makeLibraryPath runtimeLibs}" "$out/bin/aquilum-app"

    # GStreamer finds its plugins through the environment, not through
    # DT_NEEDED. Keep the wrapper's argv[0] stable so that the Wayland app id
    # and the desktop entry's StartupWMClass still match.
    wrapProgram "$out/bin/aquilum-app" \
      --argv0 aquilum-app \
      --prefix GST_PLUGIN_SYSTEM_PATH_1_0 : "${lib.makeSearchPath "lib/gstreamer-1.0" gstreamerPlugins}"
  '';

  meta = {
    description = "Fast local-first Markdown knowledge base";
    longDescription = ''
      Aquilum is a native, local-first Markdown knowledge base with a live
      preview editor, full-text search, a knowledge graph, task queries and a
      built-in book reader. Notes remain ordinary Markdown files on disk.
    '';
    homepage = "https://github.com/Freaction/Aquilum";
    changelog = "https://github.com/Freaction/Aquilum/releases";
    license = lib.licenses.agpl3Only;
    sourceProvenance = [ lib.sourceTypes.fromSource ];
    mainProgram = "aquilum-app";
    platforms = lib.platforms.linux;
  };
}
