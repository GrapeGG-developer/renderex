# Установка на Windows: открытие .rx двойным кликом

После установки расширение `.rx` регистрируется в Windows, и двойной
клик по сцене запускает интерпретатор renderex с этим файлом.

## Быстрая установка

```powershell
# из корня репозитория renderex:
powershell -ExecutionPolicy Bypass -File install\install.ps1 -Rebuild
```

Скрипт делает три вещи:

1. Собирает release-версию (`cargo build --release`), если её нет
   (флаг `-Rebuild` — пересобрать принудительно);
2. Копирует `renderex.exe` и иконку в папку установки —
   по умолчанию `%LOCALAPPDATA%\Programs\Renderex`;
3. Регистрирует расширение `.rx` в реестре.

Готово — теперь любой файл `.rx` открывается двойным кликом.

## Что именно пишется в реестр

| Ключ (HKCU\Software\Classes\...) | Значение | Зачем |
|---|---|---|
| `.rx` (default) | `Renderex.Scene` | расширение → ProgID |
| `Renderex.Scene` (default) | `Renderex scene` | описание типа файла |
| `Renderex.Scene\DefaultIcon` (default) | `...\renderex.ico` | иконка файлов .rx |
| `Renderex.Scene\shell` (default) | `open` | действие по умолчанию |
| `Renderex.Scene\shell\open` (default) | `Open in Renderex` | подпись в контекстном меню |
| `Renderex.Scene\shell\open\command` (default) | `"...\renderex.exe" "%1"` | команда запуска |

По умолчанию пишется в `HKCU` — **права администратора не нужны**,
ассоциация действует для текущего пользователя. Флаг `-Machine`
пишет в `HKLM` (для всех пользователей, требуется запуск PowerShell
от имени администратора).

## Поведение release-сборки на Windows

Release-версия собрана как GUI-приложение (`windows_subsystem =
"windows"`), поэтому при двойном клике **не появляется чёрное окно
консоли**:

- сцена компилируется, ошибки при необходимости показываются в
  диалоговом окне (MessageBox) с текстом и позицией;
- если в сцене ошибка (неизвестная команда, битый URL и т.п.) —
  тоже диалоговое окно.

Debug-сборка (`cargo run`) сохраняет консоль — так удобнее
разрабатывать: ошибки печатаются в терминал в привычном виде.

## Параметры установщика

```
powershell -ExecutionPolicy Bypass -File install\install.ps1            # установить
powershell -ExecutionPolicy Bypass -File install\install.ps1 -Rebuild   # пересобрать и установить
powershell -ExecutionPolicy Bypass -File install\install.ps1 -Machine   # для всех пользователей (админ)
powershell -ExecutionPolicy Bypass -File install\install.ps1 -Uninstall # удалить регистрацию и файлы
powershell -ExecutionPolicy Bypass -File install\install.ps1 -InstallDir "D:\Tools\Renderex"
```

## Удаление

```powershell
powershell -ExecutionPolicy Bypass -File install\install.ps1 -Uninstall
```

## Ручная регистрация (без скрипта)

То же самое командами `reg` (в `cmd`, из-под вашего пользователя):

```bat
set INSTALL=%LOCALAPPDATA%\Programs\Renderex
reg add "HKCU\Software\Classes\.rx" /ve /d "Renderex.Scene" /f
reg add "HKCU\Software\Classes\Renderex.Scene" /ve /d "Renderex scene" /f
reg add "HKCU\Software\Classes\Renderex.Scene\DefaultIcon" /ve /d "%INSTALL%\renderex.ico" /f
reg add "HKCU\Software\Classes\Renderex.Scene\shell\open\command" /ve /d "\"%INSTALL%\renderex.exe\" \"%%1\"" /f
```

Обратите внимание: если перенести `renderex.exe` в другое место,
нужно перезапустить установщик (в реестре хранится полный путь).

## Примечание

Если `.rx` уже был ассоциирован с другой программой, установка
перезапишет ассоциацию для текущего пользователя; `-Uninstall`
восстанавливает только исходное состояние реестра (стороннюю
ассоциацию не возвращает).
