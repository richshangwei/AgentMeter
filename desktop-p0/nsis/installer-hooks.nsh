; AgentMeter-specific uninstall behavior layered onto Tauri's generated NSIS flow.
; Default and silent uninstall preserve app data. Only the explicit /PURGE flag
; or Tauri's interactive delete-data checkbox enables data deletion.
!macro NSIS_HOOK_PREUNINSTALL
  ${GetOptions} $CMDLINE "/PURGE" $R9
  ${IfNot} ${Errors}
    StrCpy $DeleteAppDataCheckboxState 1
  ${EndIf}

  ; Remove the pre-0.1.0 startup name, but preserve it during an in-place update.
  ${If} $UpdateMode <> 1
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "AgentMeterP0"
  ${EndIf}
!macroend
