# GPUI Component Documentation

This is the official documentation website for [GPUI Component](https://github.com/longbridge/gpui-component) - a comprehensive UI component library for building fantastic desktop applications using GPUI.

## Getting Started

### Prerequisites

- Bun

### Installation

```bash
bun install
```

### Development

Run the development server:

```bash
bun dev
```

Open [http://localhost:3000](http://localhost:3000) with your browser to see the result.

### Build

Build the documentation for production:

```bash
bun run build
```

### Preview Production Build

```bash
bun run start
```

## Project Structure

```
doc/
├── app/                      # Next.js App Router
│   ├── (home)/              # Landing page
│   │   ├── layout.tsx
│   │   └── page.tsx
│   ├── docs/                # Documentation pages
│   │   └── [[...slug]]/
│   └── layout.tsx
├── content/                  # MDX content
│   └── docs/                # Documentation content
│       ├── index.mdx        # Docs home page
│       ├── getting-started.mdx
│       └── components/      # Component documentation
│           ├── accordion.mdx
│           ├── button.mdx
│           └── ...
├── lib/                     # Shared utilities
│   ├── source.ts           # Content source adapter
│   └── layout.shared.tsx   # Shared layout options
├── public/                  # Static assets
└── source.config.ts        # Fumadocs MDX config
```

## Documentation Content

### Adding New Pages

1. Create a new `.mdx` file in `content/docs/` or `content/docs/components/`
2. Add frontmatter with title and description:

```mdx
---
title: Your Component Name
description: Brief description of the component
---

## Overview

Your content here...
```

3. The page will be automatically added to the documentation

### Component Documentation Structure

Each component documentation should follow this structure:

- **Overview** - Brief description
- **Import** - How to import the component
- **Usage** - Basic usage examples
- **API Reference** - Complete method documentation
- **Examples** - Advanced usage patterns
- **Accessibility** - Keyboard navigation and screen reader support

## Customization

### Layout

Edit `lib/layout.shared.tsx` to customize:

- Navigation title and logo
- Navigation links
- Footer content

### Styling

The documentation uses Tailwind CSS. You can customize the theme in:

- `tailwind.config.js`
- `app/layout.tsx`

### Search

Search functionality is provided by Fumadocs and works automatically with your MDX content.

## Learn More

### GPUI Component

- [GitHub Repository](https://github.com/longbridge/gpui-component)
- [Issue Tracker](https://github.com/longbridge/gpui-component/issues)
- [Contributing Guide](https://github.com/longbridge/gpui-component/blob/main/CONTRIBUTING.md)

### Frameworks

- [Next.js Documentation](https://nextjs.org/docs)
- [Fumadocs Documentation](https://fumadocs.dev)
- [MDX Documentation](https://mdxjs.com/)

## Contributing

Contributions to the documentation are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

## License

Apache-2.0
