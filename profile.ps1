$Env:RUSTFLAGS="-Zinstrument-coverage"
$executable = cargo +nightly test --no-run --message-format=json | ConvertFrom-Json | Where-Object {$_.profile.test -eq $True} | Select-Object -Property executable
$Env:RUSTFLAGS=$Null
$executable=$executable.executable
&$executable
llvm-profdata merge default.profraw -o default.profdata
llvm-cov show $executable -Xdemangler=rustfilt "-instr-profile=default.profdata" -show-line-counts-or-regions -show-instantiations --ignore-filename-regex="(.cargo|rustc|rustup)" --format=html > result.html
llvm-cov report $executable -Xdemangler=rustfilt "-instr-profile=default.profdata" --ignore-filename-regex="(.cargo|rustc|rustup)"
