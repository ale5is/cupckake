from django.contrib import admin
from .models import Dessert


class CoreAdmin(admin.ModelAdmin):
    readonly_fields = ('created', 'updated')
# Register your models here.
admin.site.register(Dessert, CoreAdmin)