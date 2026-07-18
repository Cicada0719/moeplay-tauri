import { existsSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";

const gradlePath = path.resolve("src-tauri/gen/android/app/build.gradle.kts");
if (!existsSync(gradlePath)) throw new Error(`Android Gradle project is missing: ${gradlePath}`);

const required = [
  "MOEPLAY_ANDROID_KEYSTORE",
  "MOEPLAY_ANDROID_KEY_ALIAS",
  "MOEPLAY_ANDROID_KEY_PASSWORD",
  "MOEPLAY_ANDROID_STORE_PASSWORD",
];
for (const name of required) {
  if (!process.env[name]?.trim()) throw new Error(`${name} is required for a signed Android release`);
}

let source = readFileSync(gradlePath, "utf8");
if (source.includes("MOEPLAY_ANDROID_SIGNING")) process.exit(0);

const signing = `
    // MOEPLAY_ANDROID_SIGNING ? injected by scripts/configure-android-signing.mjs
    signingConfigs {
        create("release") {
            storeFile = file(System.getenv("MOEPLAY_ANDROID_KEYSTORE"))
            storePassword = System.getenv("MOEPLAY_ANDROID_STORE_PASSWORD")
            keyAlias = System.getenv("MOEPLAY_ANDROID_KEY_ALIAS")
            keyPassword = System.getenv("MOEPLAY_ANDROID_KEY_PASSWORD")
        }
    }
`;
const androidBlock = source.indexOf("android {");
if (androidBlock < 0) throw new Error("Unable to locate the Android Gradle android block");
source = source.slice(0, androidBlock + "android {".length) + signing + source.slice(androidBlock + "android {".length);

const releaseBlock = /buildTypes\s*\{[\s\S]*?getByName\("release"\)\s*\{/;
if (releaseBlock.test(source)) {
  source = source.replace(releaseBlock, (match) => `${match}
            signingConfig = signingConfigs.getByName("release")`);
} else {
  const buildTypesBlock = /buildTypes\s*\{/;
  if (!buildTypesBlock.test(source)) throw new Error("Unable to locate Android Gradle buildTypes block");
  source = source.replace(buildTypesBlock, `buildTypes {
        getByName("release") {
            signingConfig = signingConfigs.getByName("release")
        }`);
}
writeFileSync(gradlePath, source, "utf8");
