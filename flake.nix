{
  description = "gpui dev shell";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }: {
    devShells.x86_64-linux.default =
      let
        pkgs = import nixpkgs { system = "x86_64-linux"; };
      in
      pkgs.mkShell {
        buildInputs = with pkgs; [
          wayland
          wayland-protocols
          libxkbcommon
          pkg-config

          xorg.libX11
          xorg.libXcursor
          xorg.libXrandr
          xorg.libXi
          xorg.libXinerama
          xorg.libxcb

          vulkan-loader

          alsa-lib
          dbus
        ];

        shellHook = ''
          export LD_LIBRARY_PATH=${
            pkgs.lib.makeLibraryPath [
              pkgs.wayland
              pkgs.libxkbcommon
              pkgs.xorg.libX11
              pkgs.xorg.libXcursor
              pkgs.xorg.libXrandr
              pkgs.xorg.libXi
              pkgs.xorg.libXinerama
              pkgs.xorg.libxcb
              pkgs.vulkan-loader
              pkgs.alsa-lib
              pkgs.dbus
            ]
          }:$LD_LIBRARY_PATH
        '';
      };
  };
}