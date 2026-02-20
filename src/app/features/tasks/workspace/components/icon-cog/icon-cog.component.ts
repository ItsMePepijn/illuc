import { Component } from "@angular/core";

@Component({
    selector: "app-icon-cog",
    standalone: true,
    styles: `
        :host {
            display: inline-flex;
            align-items: center;
            justify-content: center;
            line-height: 0;
        }
        svg {
            width: 18px;
            height: 18px;
            display: block;
        }
    `,
    template: `
        <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            <circle cx="12" cy="12" r="3"></circle>
            <path
                d="M19.4 15a1.7 1.7 0 0 0 .34 1.88l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06A1.7 1.7 0 0 0 15 19.4a1.7 1.7 0 0 0-1 .16 1.7 1.7 0 0 0-.96 1.55V21a2 2 0 0 1-4 0v-.09a1.7 1.7 0 0 0-.96-1.55 1.7 1.7 0 0 0-1-.16 1.7 1.7 0 0 0-1.88.34l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.7 1.7 0 0 0 4.6 15a1.7 1.7 0 0 0-.16-1 1.7 1.7 0 0 0-1.55-.96H2.8a2 2 0 1 1 0-4h.09a1.7 1.7 0 0 0 1.55-.96 1.7 1.7 0 0 0 .16-1 1.7 1.7 0 0 0-.34-1.88l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.7 1.7 0 0 0 9 4.6a1.7 1.7 0 0 0 1-.16A1.7 1.7 0 0 0 10.96 2.89V2.8a2 2 0 1 1 4 0v.09a1.7 1.7 0 0 0 .96 1.55 1.7 1.7 0 0 0 1 .16 1.7 1.7 0 0 0 1.88-.34l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.7 1.7 0 0 0 19.4 9a1.7 1.7 0 0 0 .16 1 1.7 1.7 0 0 0 1.55.96h.09a2 2 0 1 1 0 4h-.09a1.7 1.7 0 0 0-1.55.96 1.7 1.7 0 0 0-.16 1z"
            ></path>
        </svg>
    `,
})
export class IconCogComponent {}
