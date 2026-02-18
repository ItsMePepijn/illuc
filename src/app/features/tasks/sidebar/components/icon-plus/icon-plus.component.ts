import { Component } from "@angular/core";

@Component({
    selector: "app-icon-plus",
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
            color: var(--brand-accent_strong);
        }
    `,
    template: `
        <svg
            class="create-icon"
            aria-hidden="true"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
        >
            <path d="M12 5v14"></path>
            <path d="M5 12h14"></path>
        </svg>
    `,
})
export class IconPlusComponent {}
