# next-themes ![next-themes minzip package size](https://img.shields.io/bundlephobia/minzip/next-themes) [![Version](https://img.shields.io/npm/v/next-themes.svg?colorB=green)](https://www.npmjs.com/package/next-themes)

An abstraction for themes in your React app.

- ✅ Perfect dark mode in 2 lines of code
- ✅ System setting with prefers-color-scheme
- ✅ Themed browser UI with color-scheme
- ✅ Support for Next.js 13 `appDir`
- ✅ No flash on load (both SSR and SSG)
- ✅ Sync theme across tabs and windows
- ✅ Disable flashing when changing themes
- ✅ Force pages to specific themes
- ✅ Class or data attribute selector
- ✅ `useTheme` hook

Check out the [Live Example](https://next-themes-example.vercel.app/) to try it for yourself.

## Install

```bash
$ npm install next-themes
# or
$ yarn add next-themes
```

## Use

### With pages/

You'll need a [Custom `App`](https://nextjs.org/docs/advanced-features/custom-app) to use next-themes. The simplest `_app` looks like this:

```jsx
// pages/_app.js

function MyApp({ Component, pageProps }) {
  return <Component {...pageProps} />
}

export default MyApp
```

Adding dark mode support takes 2 lines of code:

```jsx
// pages/_app.js
import { ThemeProvider } from 'next-themes'

function MyApp({ Component, pageProps }) {
  return (
    <ThemeProvider>
      <Component {...pageProps} />
    </ThemeProvider>
  )
}

export default MyApp
```

### With app/

You'll need to update your `app/layout.jsx` to use next-themes. The simplest `layout` looks like this:

```jsx
// app/layout.jsx
export default function Layout({ children }) {
  return (
    <html>
      <head />
      <body>{children}</body>
    </html>
  )
}
```

Adding dark mode support takes 2 lines of code:

```jsx
// app/layout.jsx
import { ThemeProvider } from 'next-themes'

export default function Layout({ children }) {
  return (
    <html suppressHydrationWarning>
      <head />
      <body>
        <ThemeProvider>{children}</ThemeProvider>
      </body>
    </html>
  )
}
```

Note that `ThemeProvider` is a client component, not a server component.

> **Note!** If you do not add [suppressHydrationWarning](https://reactjs.org/docs/dom-elements.html#suppresshydrationwarning:~:text=It%20only%20works%20one%20level%20deep) to your `<html>` you will get warnings because `next-themes` updates that element. This property only applies one level deep, so it won't block hydration warnings on other elements.

### HTML & CSS

That's it, your Next.js app fully supports dark mode, including System preference with `prefers-color-scheme`. The theme is also immediately synced between tabs. By default, next-themes modifies the `data-theme` attribute on the `html` element, which you can easily use to style your app:

```css
:root {
  /* Your default theme */
  --background: white;
  --foreground: black;
}

[data-theme='dark'] {
  --background: black;
  --foreground: white;
}
```

> **Note!** If you set the attribute of your Theme Provider to class for Tailwind next-themes will modify the `class` attribute on the `html` element. See [With TailwindCSS](#with-tailwindcss).

### useTheme

Your UI will need to know the current theme and be able to change it. The `useTheme` hook provides theme information:

```jsx
import { useTheme } from 'next-themes'

const ThemeChanger = () => {
  const { theme, setTheme } = useTheme()

  return (
    <div>
      The current theme is: {theme}
      <button onClick={() => setTheme('light')}>Light Mode</button>
      <button onClick={() => setTheme('dark')}>Dark Mode</button>
    </div>
  )
}
```

> **Warning!** The above code is hydration _unsafe_ and will throw a hydration mismatch warning when rendering with SSG or SSR. This is because we cannot know the `theme` on the server, so it will always be `undefined` until mounted on the client.
>
> You should delay rendering any theme toggling UI until mounted on the client. See the [example](#avoid-hydration-mismatch).

## API

Let's dig into the details.

### ThemeProvider

All your theme configuration is passed to ThemeProvider.

- `storageKey = 'theme'`: Key used to store theme setting in localStorage
- `defaultTheme = 'system'`: Default theme name (for v0.0.12 and lower the default was `light`). If `enableSystem` is false, the default theme is `light`
- `forcedTheme`: Forced theme name for the current page (does not modify saved theme settings)
- `enableSystem = true`: Whether to switch between `dark` and `light` based on `prefers-color-scheme`
- `enableColorScheme = true`: Whether to indicate to browsers which color scheme is used (dark or light) for built-in UI like inputs and buttons
- `disableTransitionOnChange = false`: Optionally disable all CSS transitions when switching themes ([example](#disable-transitions-on-theme-change))
- `themes = ['light', 'dark']`: List of theme names
- `attribute = 'data-theme'`: HTML attribute modified based on the active theme
  - accepts `class` and `data-*` (meaning any data attribute, `data-mode`, `data-color`, etc.) ([example](#class-instead-of-data-attribute))
- `value`: Optional mapping of theme name to attribute value
  - value is an `object` where key is the theme name and value is the attribute value ([example](#differing-dom-attribute-and-theme-name))
- `nonce`: Optional nonce passed to the injected `script` tag, used to allow-list the next-themes script in your CSP
- `scriptProps`: Optional props to pass to the injected `script` tag ([example](#using-with-cloudflare-rocket-loader))

### useTheme

useTheme takes no parameters, but returns:

- `theme`: Active theme name
- `setTheme(name)`: Function to update the theme. The API is identical to the [set function](https://react.dev/reference/react/useState#setstate) returned by `useState`-hook. Pass the new theme value or use a callback to set the new theme based on the current theme.
- `forcedTheme`: Forced page theme or falsy. If `forcedTheme` is set, you should disable any theme switching UI
- `resolvedTheme`: If `enableSystem` is true and the active theme is "system", this returns whether the system preference resolved to "dark" or "light". Otherwise, identical to `theme`
- `systemTheme`: If `enableSystem` is true, represents the System theme preference ("dark" or "light"), regardless what the active theme is
- `themes`: The list of themes passed to `ThemeProvider` (with "system" appended, if `enableSystem` is true)

Not too bad, right? Let's see how to use these properties with examples:

## Examples

The [Live Example](https://next-themes-example.vercel.app/) shows next-themes in action, with dark, light, system themes and pages with forced themes.

### Use System preference by default

For versions above v0.0.12, the `defaultTheme` is automatically set to "system", so to use System preference you can simply use:

```jsx
<ThemeProvider>
```

### Ignore System preference

If you don't want a System theme, disable it via `enableSystem`:

```jsx
<ThemeProvider enableSystem={false}>
```

### Class instead of data attribute

If your Next.js app uses a class to style the page based on the theme, change the attribute prop to `class`:

```jsx
<ThemeProvider attribute="class">
```

Now, setting the theme to "dark" will set `class="dark"` on the `html` element.

### Force page to a theme

Let's say your cool new marketing page is dark mode only. The page should always use the dark theme, and changing the theme should have no effect. To force a theme on your Next.js pages, simply set a variable on the page component:

```js
// pages/awesome-page.js

const Page = () => { ... }
Page.theme = 'dark'
export default Page
```

In your `_app`, read the variable and pass it to ThemeProvider:

```jsx
function MyApp({ Component, pageProps }) {
  return (
    <ThemeProvider forcedTheme={Component.theme || null}>
      <Component {...pageProps} />
    </ThemeProvider>
  )
}
```

Done! Your page is always dark theme (regardless of user preference), and calling `setTheme` from `useTheme` is now a no-op. However, you should make sure to disable any of your UI that would normally change the theme:

```js
const { forcedTheme } = useTheme()

// Theme is forced, we shouldn't allow user to change the theme
const disabled = !!forcedTheme
```

### Disable transitions on theme change

I wrote about [this technique here](https://paco.sh/blog/disable-theme-transitions). We can forcefully disable all CSS transitions before the theme is changed, and re-enable them immediately afterwards. This ensures your UI with different transition durations won't feel inconsistent when changing the theme.

To enable this behavior, pass the `disableTransitionOnChange` prop:

```jsx
<ThemeProvider disableTransitionOnChange>
```

### Differing DOM attribute and theme name

The name of the active theme is used as both the localStorage value and the value of the DOM attribute. If the theme name is "pink", localStorage will contain `theme=pink` and the DOM will be `data-theme="pink"`. You **cannot** modify the localStorage value, but you **can** modify the DOM value.

If we want the DOM to instead render `data-theme="my-pink-theme"` when the theme is "pink", pass the `value` prop:

```jsx
<ThemeProvider value={{ pink: 'my-pink-theme' }}>
```

Done! To be extra clear, this affects only the DOM. Here's how all the values will look:

```js
const { theme } = useTheme()
// => "pink"

localStorage.getItem('theme')
// => "pink"

document.documentElement.getAttribute('data-theme')
// => "my-pink-theme"
```

### Using with Cloudflare Rocket Loader

[Rocket Loader](https://developers.cloudflare.com/fundamentals/speed/rocket-loader/) is a Cloudflare optimization that defers the loading of inline and external scripts to prioritize the website content. Since next-themes relies on a script injection to avoid screen flashing on page load, Rocket Loader breaks this functionality. Individual scripts [can be ignored](https://developers.cloudflare.com/fundamentals/speed/rocket-loader/ignore-javascripts/) by adding the `data-cfasync="false"` attribute to the script tag:

```jsx
<ThemeProvider scriptProps={{ 'data-cfasync': 'false' }}>
```

### More than light and dark mode

next-themes is designed to support any number of themes! Simply pass a list of themes:

```jsx
<ThemeProvider themes={['pink', 'red', 'blue']}>
```

> **Note!** When you pass `themes`, the default set of themes ("light" and "dark") are overridden. Make sure you include those if you still want your light and dark themes:

```jsx
<ThemeProvider themes={['pink', 'red', 'blue', 'light', 'dark']}>
```

For an example on how to use this, check out the [multi-theme example](./examples/multi-theme/README.md)

### Without CSS variables

This library does not rely on your theme styling using CSS variables. You can hard-code the values in your CSS, and everything will work as expected (without any flashing):

```css
html,
body {
  color: #000;
  background: #fff;
}

[data-theme='dark'],
[data-theme='dark'] body {
  color: #fff;
  background: #000;
}
```

### With Styled Components and any CSS-in-JS

Next Themes is completely CSS independent, it will work with any library. For example, with Styled Components you just need to `createGlobalStyle` in your custom App:

```jsx
// pages/_app.js
import { createGlobalStyle } from 'styled-components'
import { ThemeProvider } from 'next-themes'

// Your themeing variables
const GlobalStyle = createGlobalStyle`
  :root {
    --fg: #000;
    --bg: #fff;
  }

  [data-theme="dark"] {
    --fg: #fff;
    --bg: #000;
  }
`

function MyApp({ Component, pageProps }) {
  return (
    <>
      <GlobalStyle />
      <ThemeProvider>
        <Component {...pageProps} />
      </ThemeProvider>
    </>
  )
}
```

### Avoid Hydration Mismatch

Because we cannot know the `theme` on the server, many of the values returned from `useTheme` will be `undefined` until mounted on the client. This means if you try to render UI based on the current theme before mounting on the client, you will see a hydration mismatch error.

The following code sample is **unsafe**:

```jsx
import { useTheme } from 'next-themes'

// Do NOT use this! It will throw a hydration mismatch error.
const ThemeSwitch = () => {
  const { theme, setTheme } = useTheme()

  return (
    <select value={theme} onChange={e => setTheme(e.target.value)}>
      <option value="system">System</option>
      <option value="dark">Dark</option>
      <option value="light">Light</option>
    </select>
  )
}

export default ThemeSwitch
```

To fix this, make sure you only render UI that uses the current theme when the page is mounted on the client:

```jsx
import { useState, useEffect } from 'react'
import { useTheme } from 'next-themes'

const ThemeSwitch = () => {
  const [mounted, setMounted] = useState(false)
  const { theme, setTheme } = useTheme()

  // useEffect only runs on the client, so now we can safely show the UI
  useEffect(() => {
    setMounted(true)
  }, [])

  if (!mounted) {
    return null
  }

  return (
    <select value={theme} onChange={e => setTheme(e.target.value)}>
      <option value="system">System</option>
      <option value="dark">Dark</option>
      <option value="light">Light</option>
    </select>
  )
}

export default ThemeSwitch
```

Alternatively, you could lazy load the component on the client side. The following example uses `next/dynamic` but you could also use `React.lazy`:

```js
import dynamic from 'next/dynamic'

const ThemeSwitch = dynamic(() => import('./ThemeSwitch'), { ssr: false })

const ThemePage = () => {
  return (
    <div>
      <ThemeSwitch />
    </div>
  )
}

export default ThemePage
```

To avoid [Layout Shift](https://web.dev/cls/), consider rendering a skeleton/placeholder until mounted on the client side.

#### Images

Showing different images based on the current theme also suffers from the hydration mismatch problem. With [`next/image`](https://nextjs.org/docs/basic-features/image-optimization) you can use an empty image until the theme is resolved:

```jsx
import Image from 'next/image'
import { useTheme } from 'next-themes'

function ThemedImage() {
  const { resolvedTheme } = useTheme()
  let src

  switch (resolvedTheme) {
    case 'light':
      src = '/light.png'
      break
    case 'dark':
      src = '/dark.png'
      break
    default:
      src = 'data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7'
      break
  }

  return <Image src={src} width={400} height={400} />
}

export default ThemedImage
```

#### CSS

You can also use CSS to hide or show content based on the current theme. To avoid the hydration mismatch, you'll need to render _both_ versions of the UI, with CSS hiding the unused version. For example:

```jsx
function ThemedImage() {
  return (
    <>
      {/* When the theme is dark, hide this div */}
      <div data-hide-on-theme="dark">
        <Image src="light.png" width={400} height={400} />
      </div>

      {/* When the theme is light, hide this div */}
      <div data-hide-on-theme="light">
        <Image src="dark.png" width={400} height={400} />
      </div>
    </>
  )
}

export default ThemedImage
```

```css
[data-theme='dark'] [data-hide-on-theme='dark'],
[data-theme='light'] [data-hide-on-theme='light'] {
  display: none;
}
```

### With TailwindCSS

[Visit the live example](https://next-themes-tailwind.vercel.app) • [View the example source code](https://github.com/pacocoursey/next-themes/tree/master/examples/tailwind)

> NOTE! Tailwind only supports dark mode in version >2.

In your `tailwind.config.js`, set the dark mode property to `selector`:

```js
// tailwind.config.js
module.exports = {
  darkMode: 'selector'
}
```

_Note: If you are using an older version of tailwindcss < 3.4.1 use `'class'` instead of `'selector'`_

Set the attribute for your Theme Provider to class:

```tsx
// pages/_app.tsx
<ThemeProvider attribute="class">
```

If you're using the value prop to specify different attribute values, make sure your dark theme explicitly uses the "dark" value, as required by Tailwind.

That's it! Now you can use dark-mode specific classes:

```tsx
<h1 className="text-black dark:text-white">
```

#### Using a custom selector (tailwindcss > 3.4.1)

Tailwind also allows you to use a [custom selector](https://tailwindcss.com/docs/dark-mode#customizing-the-selector) for dark-mode as of v3.4.1.

In that case, your `tailwind.config.js` would look like this:

```js
// tailwind.config.js
module.exports = {
  // data-mode is used as an example, next-themes supports using any data attribute
  darkMode: ['selector', '[data-mode="dark"]']
  …
}
```

Now set the attribute for your ThemeProvider to `data-mode`:

```tsx
// pages/_app.tsx
<ThemeProvider attribute="data-mode">
```

With this setup, you can now use Tailwind's dark mode classes, as in the previous example:

## Discussion

### The Flash

ThemeProvider automatically injects a script into `next/head` to update the `html` element with the correct attributes before the rest of your page loads. This means the page will not flash under any circumstances, including forced themes, system theme, multiple themes, and incognito. No `noflash.js` required.

## FAQ

---

**Why is my page still flashing?**

In Next.js dev mode, the page may still flash. When you build your app in production mode, there will be no flashing.

---

**Why do I get server/client mismatch error?**

When using `useTheme`, you will use see a hydration mismatch error when rendering UI that relies on the current theme. This is because many of the values returned by `useTheme` are undefined on the server, since we can't read `localStorage` until mounting on the client. See the [example](#avoid-hydration-mismatch) for how to fix this error.

---

**Do I need to use CSS variables with this library?**

Nope. See the [example](#without-css-variables).

---

**Can I set the class or data attribute on the body or another element?**

Nope. If you have a good reason for supporting this feature, please open an issue.

---

**Can I use this package with Gatsby or CRA?**

Yes, starting from the 0.3.0 version.

---

**Is the injected script minified?**

Yes.

---

**Why is `resolvedTheme` necessary?**

When supporting the System theme preference, you want to make sure that's reflected in your UI. This means your buttons, selects, dropdowns, or whatever you use to indicate the current theme should say "System" when the System theme preference is active.

If we didn't distinguish between `theme` and `resolvedTheme`, the UI would show "Dark" or "Light", when it should really be "System".

`resolvedTheme` is then useful for modifying behavior or styles at runtime:

```jsx
const { resolvedTheme } = useTheme()

<div style={{ color: resolvedTheme === 'dark' ? 'white' : 'black' }}>
```

If we didn't have `resolvedTheme` and only used `theme`, you'd lose information about the state of your UI (you would only know the theme is "system", and not what it resolved to).
