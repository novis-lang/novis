Returns `true` when the push service no longer knows the subscription. The status is then `404`
or `410`.

A subscription is gone when the user removed the permission for notifications, or when the
browser created a new subscription. The push service then deleted the old one. When `gone()` is
`true`, delete the subscription from your database too. Messages to it will never arrive. For
every other status, `gone()` is `false`. This includes an error such as `500` or `429`, because the
subscription still exists.

**Good to know:** the examples use `Core\Test::answerHttp` to give fixed replies, so they run
without a network. They check one subscription that is gone, show which status codes make
`gone()` true, and delete the old subscriptions from a list.
