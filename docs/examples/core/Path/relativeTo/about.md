Returns the path that leads from one folder to a file or another folder. PHP has no function for
this. The method works on the text of the paths and never looks at the disk.

The result goes up with `..` out of each folder the two paths do not share, and then down to the
place you asked for. From `/srv/shop/views`, the path to `/srv/shop/logs/app.log` is
`../logs/app.log`. When both paths name the same place, the result is `.`. Both paths have their
`.` and `..` parts removed first, as `Core\Path::normalize` does.

The result is `null` when no relative path exists. That happens when one path is absolute and the
other is not, or when they start at different roots, such as two drives.

**The examples below** find a relative path, show when the result is `null`, and write the links
between the pages of a website.
