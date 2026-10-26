$JdkVersion = 21
$CmdlineTools = '16111833'
$CompileSdk = 36
$BuildTools = '36.0.0'
$NdkVersion = '29.0.14206865'
$RustTargets = @('aarch64-linux-android', 'x86_64-linux-android')
$GradleVersion = '9.6.0'

$AndroidRoot = if ($env:AQUILUM_ANDROID_ROOT) { $env:AQUILUM_ANDROID_ROOT } else { "$env:LOCALAPPDATA\Android" }
$env:ANDROID_HOME = "$AndroidRoot\Sdk"
$env:ANDROID_SDK_ROOT = $env:ANDROID_HOME
$env:ANDROID_NDK_HOME = "$env:ANDROID_HOME\ndk\$NdkVersion"
$env:ANDROID_NDK_ROOT = $env:ANDROID_NDK_HOME
$env:JAVA_HOME = "$AndroidRoot\jdk-$JdkVersion"
$Gradle = "$AndroidRoot\gradle-$GradleVersion\bin\gradle.bat"
$env:PATH ="$env:JAVA_HOME\bin;$env:ANDROID_HOME\platform-tools;$env:PATH"
