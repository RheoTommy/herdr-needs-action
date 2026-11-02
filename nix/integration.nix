{
  runCommand,
  bashInteractive,
  coreutils,
  gnugrep,
  jq,
  herdr,
  plugin,
}:

runCommand "herdr-needs-action-integration"
  {
    nativeBuildInputs = [
      bashInteractive
      coreutils
      gnugrep
      herdr
      jq
    ];
    # The shell herdr starts in each pane.
    SHELL = "${bashInteractive}/bin/bash";
  }
  ''
    bash ${../tests/integration.sh} ${plugin}/${plugin.pluginPath}
    touch $out
  ''
