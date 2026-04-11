import { Component } from "@angular/core";

@Component({
    selector: "app-icon-more-horizontal",
    standalone: true,
    styles: `
        :host {
            display: inline-flex;
            align-items: center;
            justify-content: center;
            line-height: 0;
        }
        svg {
            width: 14px;
            height: 14px;
            display: block;
        }
    `,
    template: `
        <svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
            <circle cx="6" cy="12" r="1.75" />
            <circle cx="12" cy="12" r="1.75" />
            <circle cx="18" cy="12" r="1.75" />
        </svg>
    `,
})
export class IconMoreHorizontalComponent {}
