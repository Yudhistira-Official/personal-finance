#!/usr/bin/env bash
# Inject Android release signing config into the Tauri-generated Gradle file.
# Called AFTER `tauri android init` (which regenerates build.gradle.kts).
set -euo pipefail

GRADLE="src-tauri/gen/android/app/build.gradle.kts"
KS="src-tauri/gen/android/keystore.properties"

if [ ! -f "$GRADLE" ]; then
  echo "ERROR: $GRADLE not found — run 'tauri android init' first."
  exit 1
fi

# Only patch if not already patched (idempotent).
if grep -q 'signingConfigs' "$GRADLE"; then
  echo "signingConfigs already present — skip patch."
  exit 0
fi

# 1) Add import.
if ! grep -q 'import java.io.FileInputStream' "$GRADLE"; then
  sed -i '1i import java.io.FileInputStream' "$GRADLE"
fi

# 2) Insert signingConfigs block right before the "buildTypes" block.
python3 - "$GRADLE" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
anchor = "    buildTypes {"
inject = '''    signingConfigs {
        create("release") {
            val keystorePropertiesFile = rootProject.file("keystore.properties")
            val keystoreProperties = Properties()
            if (keystorePropertiesFile.exists()) {
                keystoreProperties.load(FileInputStream(keystorePropertiesFile))
            }
            keyAlias = keystoreProperties["keyAlias"] as String
            keyPassword = keystoreProperties["password"] as String
            storeFile = file(keystoreProperties["storeFile"] as String)
            storePassword = keystoreProperties["password"] as String
        }
    }

'''
if "signingConfigs" not in s:
    s = s.replace(anchor, inject + anchor, 1)
open(p, "w").write(s)
PY

# 3) Apply signingConfig to the release buildType.
python3 - "$GRADLE" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
old = '''        getByName("release") {
            isMinifyEnabled = true'''
new = '''        getByName("release") {
            signingConfig = signingConfigs.getByName("release")
            isMinifyEnabled = true'''
if "signingConfig = signingConfigs.getByName(\"release\")" not in s:
    s = s.replace(old, new, 1)
open(p, "w").write(s)
PY

echo "Patched $GRADLE with release signingConfig."
