@echo off
setlocal
rem Cargo does not apply target RUSTFLAGS to host build scripts during cross
rem compilation. Give only those host invocations the matching bundled GNU CRT.
rem Cargo supplies the compiler executable as the first, quoted argument.
set "scotoma_is_host=1"
for %%A in (%*) do if "%%~A"=="--target" set "scotoma_is_host="
for %%A in (%*) do if "%%~A"=="--target=aarch64-linux-android" set "scotoma_is_host="
for %%A in (%*) do if "%%~A"=="--target=armv7-linux-androideabi" set "scotoma_is_host="
for %%A in (%*) do if "%%~A"=="--target=x86_64-linux-android" set "scotoma_is_host="
if not defined scotoma_is_host goto target
%* -C linker=x86_64-w64-mingw32-clang.exe -C link-self-contained=yes -C dlltool=llvm-dlltool.exe
exit /b %errorlevel%
:target
%*
exit /b %errorlevel%
