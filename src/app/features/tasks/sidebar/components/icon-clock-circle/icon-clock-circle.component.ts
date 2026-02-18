import { Component } from "@angular/core";

@Component({
    selector: "app-icon-clock-circle",
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
            class="nav-icon"
            aria-hidden="true"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
        >
            <circle cx="12" cy="12" r="9"></circle>
            <path d="M12 7v5l3 2"></path>
        </svg>
    `,
})
export class IconClockCircleComponent {}
