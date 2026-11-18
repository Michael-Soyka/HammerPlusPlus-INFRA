<p align="center"><img width="1090" height="256" alt="image" src=".github/images/hpp-patcher-infra-banner.png" /></p>

[English Version](README.md) | Русская версия

## Установка

**Hammer++ должен быть закрыт.**

1. Установите [INFRA](https://store.steampowered.com/app/251110/INFRA/);
2. Скачайте [Hammer++ для Portal 2](https://ficool2.github.io/HammerPlusPlus-Website/download.html) и распакуйте его рядом с патчером;
3. Запустите **infra-hpp-patcher.exe** (если игра установлена, а папка Hammer++ для Portal 2 лежит рядом с патчером, они подставятся автоматически, иначе укажите их вручную);
4. Нажмите **Install**;

Опционально: вы можете установить совместимую с этим патчером тёмную тему [Extra Dark](https://github.com/Michael-Soyka/HammerPlusPlus-ExtraDark).

## Показ

<img width="1920" height="1070" alt="image" src=".github/images/INFRA.png" />
<p align="center">Hammer++ - INFRA</p>

[Дизайн на Figma](https://www.figma.com/design/zUpakDBp6vfcbPzCPseVBF/Source-Mini-Tools?node-id=0-1&t=rR9ijBfBiP36KZYt-1)

## Требования

### Общие

- INFRA из Steam
- Hammer++ для Portal 2, 32-битный: патчер принимает только эту сборку

### Для разработчиков

- Rust, stable, с 32-битной целью: `rustup target add i686-pc-windows-msvc`
- Visual Studio \>=2022 или Build Tools с компонентом "Разработка классических приложений на C++": компоновщик MSVC и Windows SDK

## Сборка

```
cargo build --release
```

На выходе `target\i686-pc-windows-msvc\release\infra-hpp-patcher.exe`, внутри `infra_hpp_adapter.dll`. Всё собирается 32-битным, как и сам Hammer++: цель задана в `.cargo\config.toml`. Скрипт сборки патчера сам собирает release-версию адаптера и встраивает её.

Адреса патчера — в [ADDRESSES.md](ADDRESSES.md).

## Авторы

- Loiste Interactive - Создатели [INFRA](https://loisteinteractive.com)
- Ficool2 - Создатель [Hammer++](https://ficool2.github.io/HammerPlusPlus-Website/) :blue_heart:
- Michael Soyka (Moon-6) - Структура, основные моменты, дизайн, тесты
- Claude - Основной код патчера
