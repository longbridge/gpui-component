import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';

/**
 * Shared layout configurations
 *
 * you can customise layouts individually from:
 * Home Layout: app/(home)/layout.tsx
 * Docs Layout: app/docs/layout.tsx
 */
export function baseOptions(): BaseLayoutProps {
  return {
    nav: {
      title: (
        <>
          <svg
            width="24"
            height="24"
            viewBox="0 0 24 24"
            xmlns="http://www.w3.org/2000/svg"
            aria-label="GPUI Component Logo"
            className="mr-2"
          >
            <rect width="24" height="24" rx="4" fill="currentColor" opacity="0.1" />
            <path
              d="M6 8h12M6 12h12M6 16h8"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
            />
          </svg>
          <span className="font-semibold">GPUI Component</span>
        </>
      ),
    },
    links: [
      {
        text: 'Documentation',
        url: '/docs',
        active: 'nested-url',
      },
      {
        text: 'Components',
        url: '/docs/components/accordion',
      },
      {
        text: 'GitHub',
        url: 'https://github.com/longbridge/gpui-component',
        external: true,
      },
    ],
  };
}
