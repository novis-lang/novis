# Paperwork

A small web app written in Novis. It makes three PDF documents, the way a business needs them:

- an invoice over three pages, with a QR code for the payment and the electronic invoice inside,
- a confidential quarterly report with a password, a table of contents and charts,
- the event tickets of one order, with a QR code and a barcode on each ticket.

Run these commands in this folder:

```sh
nvs serve public/index.nvs   # open http://127.0.0.1:8143/
nvs test .                   # runs the tests
```

In a clone of the repository, `bun nv app paperwork` starts the app from any folder.

The password of the report is `novis`.

The page [Example apps: Paperwork](https://novis-lang.org/guides/example-apps/paperwork/) explains each file.
