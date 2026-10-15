; V105-20 cyex deck's installer additions (Tauri's NSIS hooks): keep the old program until
; the new one is in place, and ask before closing a running app.

!macro NSIS_HOOK_PREINSTALL
  ; Runs before any file is written. Copying a running exe works.
  ${If} ${FileExists} "$INSTDIR\${MAINBINARYNAME}.exe"
    CopyFiles /SILENT "$INSTDIR\${MAINBINARYNAME}.exe" "$INSTDIR\${MAINBINARYNAME}.exe.old"
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  Delete "$INSTDIR\${MAINBINARYNAME}.exe.old"
!macroend

; A failed install puts the old program back, so the old version keeps working. This file is
; included before the template defines MAINBINARYNAME, so the name is written out.
Function .onInstFailed
  ${If} ${FileExists} "$INSTDIR\cyex-deck.exe.old"
    Delete "$INSTDIR\cyex-deck.exe"
    Rename "$INSTDIR\cyex-deck.exe.old" "$INSTDIR\cyex-deck.exe"
  ${EndIf}
FunctionEnd

; Asks to close a running cyex deck instead of killing it. Passive and silent installs
; (the in-app update) close it the way Tauri does.
!macro CyexCloseApp executablePath
  !define CyexID ${__LINE__}
  cyex_check_${CyexID}:
  !insertmacro RestartManager_StartSession $R0
  ${If} $R0 != ""
    !insertmacro RestartManager_RegisterFile $R0 "${executablePath}"
    ${If} $0 = 0
      System::Call 'RSTRTMGR::RmGetList(p R0, *i .r1, *i .r2, p 0, *i .r3) i .r0'
      ${If} $0 = ${ERROR_MORE_DATA}
        ${If} $PassiveMode = 1
        ${OrIf} ${Silent}
          System::Call 'RSTRTMGR::RmShutdown(p R0, i ${RmForceShutdown}, p 0) i .r0'
        ${Else}
          !insertmacro RestartManager_EndSession $R0
          MessageBox MB_RETRYCANCEL|MB_ICONINFORMATION "cyex deck is open. Close it, then click Retry." IDRETRY cyex_check_${CyexID}
          ; Nothing was written yet: the safety copy isn't needed.
          Delete "$INSTDIR\${MAINBINARYNAME}.exe.old"
          Abort "cyex deck is still open. Nothing was changed."
        ${EndIf}
      ${EndIf}
    ${EndIf}
    !insertmacro RestartManager_EndSession $R0
  ${EndIf}
  !undef CyexID
!macroend
