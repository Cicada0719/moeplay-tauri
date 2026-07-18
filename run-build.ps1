$env:JAVA_HOME='C:\Program Files\Microsoft\jdk-17.0.19.10-hotspot'
$env:ANDROID_HOME='C:\Users\sgy\AppData\Local\Android\Sdk'
Set-Location 'D:\我的文件\桌面备份\hermes\moeplay-tauri\src-tauri\gen\android'
& .\gradlew.bat assembleArm64Debug -x rustBuildArm64Debug --console=plain
exit $LASTEXITCODE
