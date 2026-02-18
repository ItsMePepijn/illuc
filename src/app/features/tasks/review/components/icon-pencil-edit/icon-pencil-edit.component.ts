import { Component } from "@angular/core";

@Component({
    selector: "app-icon-pencil-edit",
    standalone: true,
    styles: `
        :host {
            display: inline-flex;
            align-items: center;
            justify-content: center;
            line-height: 0;
            font-size: 16px;
        }
        svg {
            width: 1em;
            height: 1em;
            display: block;
        }
    `,
    template: `
        <svg
            class="action-icon"
            viewBox="0 0 24 24"
            aria-hidden="true"
        >
            <path
                d="M4 20h4l10-10-4-4L4 16v4zm13.7-11.3 1.6-1.6a1 1 0 0 0 0-1.4l-1.3-1.3a1 1 0 0 0-1.4 0L15 6.1l2.7 2.6z"
                fill="currentColor"
            />
        </svg>
    `,
})
export class IconPencilEditComponent {}
