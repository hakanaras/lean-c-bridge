# Make sure libclang.dylib is discoverable:
export DYLD_LIBRARY_PATH=/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib/

cargo run -- \
  --clang-arg='-I/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include' \
  --clang-arg='-I/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/System/Library/Frameworks' \
  --clang-arg='-I/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/include' \
  --clang-arg='-I/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib/clang/16/include' \
  --function-blacklist='default_blacklist' \
  --lean-module-name='Test' \
  --output-dir='./test/Lean/' \
  --interface-choices='./test/interface-choices.json' \
  'test/test.h'
  # --ui \

cd test/Lean
lake clean
lake exec Main